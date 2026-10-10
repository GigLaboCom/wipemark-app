//! A scripted HTTP/1.1 server on `127.0.0.1:0` — the only network any test
//! of this crate touches (E2-3 §4.6).
//!
//! Each connection takes the next [`Reply`] in the script: a status, its
//! headers, and a body written in chunks with optional pauses, or cut off
//! after so many bytes. Every response is `Connection: close` with no
//! `Content-Length`, so the body ends where the connection does — which is
//! what makes "the server dropped it half way" expressible at all. Every
//! request is recorded (method, path, headers, body) for the assertions,
//! and every accepted connection is counted, so a test can say a socket was
//! never opened. A connection past the end of the script is answered `500
//! unscripted` and recorded like the rest.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// One scripted response.
#[derive(Debug, Clone, Default)]
pub(crate) struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    /// Silence before the status line.
    late: Duration,
    /// The body: each piece written after its pause.
    chunks: Vec<(Duration, Vec<u8>)>,
    /// Close the connection after this many bytes of body.
    drop_after: Option<usize>,
}

impl Reply {
    pub(crate) fn status(status: u16) -> Self {
        Self {
            status,
            ..Self::default()
        }
    }

    /// `{origin}` in `value` is this server's `127.0.0.1:<port>`, filled
    /// in when the reply is written — for a `Location` pointing back here.
    pub(crate) fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    pub(crate) fn late(mut self, pause: Duration) -> Self {
        self.late = pause;
        self
    }

    pub(crate) fn chunk(self, bytes: impl Into<Vec<u8>>) -> Self {
        self.after(Duration::ZERO, bytes)
    }

    pub(crate) fn after(mut self, pause: Duration, bytes: impl Into<Vec<u8>>) -> Self {
        self.chunks.push((pause, bytes.into()));
        self
    }

    pub(crate) fn drop_after(mut self, bytes: usize) -> Self {
        self.drop_after = Some(bytes);
        self
    }

    /// A 200 of server-sent events, one event per chunk, each a little
    /// after the last so they arrive as separate reads.
    pub(crate) fn sse(events: &[String]) -> Self {
        let mut reply = Self::status(200).header("Content-Type", "text/event-stream");
        for event in events {
            reply = reply.after(Duration::from_millis(5), format!("data: {event}\n\n"));
        }
        reply
    }

    /// A 200 of newline-delimited JSON, one line per chunk.
    pub(crate) fn ndjson(lines: &[String]) -> Self {
        let mut reply = Self::status(200).header("Content-Type", "application/x-ndjson");
        for line in lines {
            reply = reply.after(Duration::from_millis(5), format!("{line}\n"));
        }
        reply
    }
}

/// One request as it arrived.
#[derive(Debug, Clone)]
pub(crate) struct Recorded {
    pub method: String,
    pub path: String,
    /// Names lowercased.
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Recorded {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The body as JSON.
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).expect("the request body is JSON")
    }
}

/// The server, while the test runs.
pub(crate) struct FakeServer {
    address: SocketAddr,
    recorded: Arc<Mutex<Vec<Recorded>>>,
    connections: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

impl FakeServer {
    pub(crate) fn start(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        listener
            .set_nonblocking(true)
            .expect("a non-blocking listener");
        let address = listener.local_addr().expect("the port");
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let server = FakeServer {
            address,
            recorded: Arc::clone(&recorded),
            connections: Arc::clone(&connections),
            stop: Arc::clone(&stop),
        };
        std::thread::spawn(move || {
            let mut script: VecDeque<Reply> = replies.into();
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        connections.fetch_add(1, Ordering::SeqCst);
                        let reply = script
                            .pop_front()
                            .unwrap_or_else(|| Reply::status(500).chunk("unscripted request"));
                        let recorded = Arc::clone(&recorded);
                        std::thread::spawn(move || answer(stream, address, &reply, &recorded));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        server
    }

    /// `http://127.0.0.1:<port><path>`.
    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin())
    }

    pub(crate) fn origin(&self) -> String {
        format!("http://{}", self.address)
    }

    pub(crate) fn port(&self) -> u16 {
        self.address.port()
    }

    pub(crate) fn requests(&self) -> Vec<Recorded> {
        self.recorded
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn answer(
    mut stream: TcpStream,
    address: SocketAddr,
    reply: &Reply,
    recorded: &Mutex<Vec<Recorded>>,
) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    if let Some(request) = read_request(&mut stream) {
        recorded
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(request);
    }
    std::thread::sleep(reply.late);
    let mut head = format!("HTTP/1.1 {} {}\r\n", reply.status, reason(reply.status));
    for (name, value) in &reply.headers {
        let value = value.replace("{origin}", &address.to_string());
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    let _ = stream.flush();
    let mut written = 0usize;
    for (pause, bytes) in &reply.chunks {
        std::thread::sleep(*pause);
        let bytes = match reply.drop_after {
            Some(limit) if written + bytes.len() > limit => &bytes[..limit - written],
            _ => &bytes[..],
        };
        if stream.write_all(bytes).is_err() {
            return;
        }
        let _ = stream.flush();
        written += bytes.len();
        if reply.drop_after.is_some_and(|limit| written >= limit) {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}

fn read_request(stream: &mut TcpStream) -> Option<Recorded> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    let end = loop {
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break end;
        }
        let read = stream.read(&mut buffer).ok()?;
        if read == 0 {
            return None;
        }
        bytes.extend_from_slice(&buffer[..read]);
    };
    let head = String::from_utf8_lossy(&bytes[..end]).to_string();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next()?.split(' ');
    let method = request_line.next()?.to_owned();
    let path = request_line.next()?.to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = bytes[end + 4..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read]);
    }
    Some(Recorded {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        301 => "Moved Permanently",
        302 => "Found",
        307 => "Temporary Redirect",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Status",
    }
}
