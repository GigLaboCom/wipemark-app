//! The road to the running application (D52, E5-2).
//!
//! When the application runs, its MCP server has the model on duty —
//! loaded already, or loaded by the application's own keep policy — and a
//! `rewrite` asked of this command is served there rather than by a second
//! copy loaded here. The server leaves a beacon under the data directory
//! while it listens (`wipemark_models::Beacon`); this module reads it and
//! trusts it only after three checks, in this order (H14):
//!
//! 1. the address is **loopback** — a beacon naming anything else is never
//!    dialled, whoever wrote it (D52: a non-loopback bind is never used
//!    for this);
//! 2. the process it names is **running** — a crash leaves the file
//!    behind, and a stale beacon is the ordinary case;
//! 3. the server there answers `initialize` as **wipemark** — a process id
//!    can be reused.
//!
//! Any of them failing means there is no application to use, and the
//! command loads its own engine. Once the `tools/call` has been sent, its
//! answer is the answer: a connection lost after that is said, and nothing
//! is run a second time here.
//!
//! The transport is the server's own: one `POST /mcp` with a JSON body,
//! one JSON answer, `Connection: close` — a dozen lines of `std::net`
//! rather than an HTTP client for one loopback call. The answer is read
//! in short waits so that Ctrl-C can hang up: the application sees the
//! connection close and cancels the job (H11).

use std::io::{self, ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use wipemark_models::{Beacon, Layout};

/// How long a knock may take: the connection, and the answer to
/// `initialize`. Loopback answers in a millisecond; a server that has not
/// answered in two seconds is not one to hand a document to.
const KNOCK: Duration = Duration::from_secs(2);

/// How long the answer is waited for between two looks at the socket (and
/// at Ctrl-C and the patience).
///
/// A look, not a read deadline: the socket is non-blocking while the answer
/// is awaited. Windows documents a receive that times out under
/// `SO_RCVTIMEO` as leaving the connection "in an indeterminate state" that
/// "should be closed", and reading on after one is what made the CLI say
/// the application had stopped answering, now and then, on a loaded
/// Windows runner (`a_chunk_that_kept_its_source_exits_three`,
/// `rewrite_uses_the_running_application`, 2026-11-16).
const TICK: Duration = Duration::from_millis(20);

/// The running application, found and asked who it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct App {
    pub address: SocketAddr,
}

/// Why a call came back with no answer.
#[derive(Debug)]
pub(crate) enum Unheard {
    /// Nobody took the connection. Nothing was sent.
    Unreachable(io::Error),
    /// The connection went before the answer was whole — after the call
    /// was sent.
    Lost(io::Error),
    /// Ctrl-C: this side hung up, and the application cancels the job.
    Interrupted,
    /// Something answered that is not the server's answer: another status
    /// than 200, or a body that is not JSON-RPC. The words are for a
    /// sentence on stderr.
    Garbled(String),
}

/// The running application, if there is one this command may use.
pub(crate) fn find(layout: &Layout) -> Option<App> {
    let beacon = Beacon::read(&layout.beacon_path())?;
    let Some(address) = beacon.reachable() else {
        tracing::warn!("the beacon names an address off this machine; it is not dialled");
        return None;
    };
    if !beacon.alive() {
        tracing::info!(
            pid = beacon.pid,
            "the beacon outlived its process; the application is not running"
        );
        return None;
    }
    let hello = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "wipemark-cli", "version": env!("CARGO_PKG_VERSION") },
        },
    });
    match exchange(address, &hello, Some(KNOCK), &AtomicBool::new(false)) {
        Ok(answer) if answer["result"]["serverInfo"]["name"] == "wipemark" => {
            tracing::info!(port = address.port(), "the running application answered");
            Some(App { address })
        }
        Ok(_) => {
            tracing::warn!(
                port = address.port(),
                "something else answers on the beacon's port"
            );
            None
        }
        Err(error) => {
            tracing::info!(?error, "the beacon's server did not answer");
            None
        }
    }
}

/// `tools/call rewrite` with `arguments`, and the result — the tool's
/// answer, an `isError` refusal included. Waits as long as the job takes;
/// `interrupted` hangs up. `meta` is the params' `_meta` (MCP's own place
/// for what is not an argument): that this command is asking, and what the
/// document is called, for the application's journal row (E4-6b, R4).
pub(crate) fn rewrite(
    app: App,
    arguments: &Value,
    meta: &Value,
    interrupted: &AtomicBool,
) -> Result<Value, Unheard> {
    let call = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "rewrite", "arguments": arguments, "_meta": meta },
    });
    let answer = exchange(app.address, &call, None, interrupted)?;
    if let Some(error) = answer.get("error") {
        return Err(Unheard::Garbled(
            error["message"]
                .as_str()
                .unwrap_or("a JSON-RPC error")
                .to_owned(),
        ));
    }
    match answer.get("result") {
        Some(result) => Ok(result.clone()),
        None => Err(Unheard::Garbled("an answer with no result".to_owned())),
    }
}

/// One request, one answer. `patience` bounds the wait for the answer;
/// `None` waits for as long as the server takes.
fn exchange(
    address: SocketAddr,
    request: &Value,
    patience: Option<Duration>,
    interrupted: &AtomicBool,
) -> Result<Value, Unheard> {
    let mut stream = TcpStream::connect_timeout(&address, KNOCK).map_err(Unheard::Unreachable)?;
    let body = request.to_string();
    let head = format!(
        "POST /mcp HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\n\
         Accept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .set_write_timeout(Some(KNOCK))
        .and_then(|()| stream.write_all(head.as_bytes()))
        .and_then(|()| stream.write_all(body.as_bytes()))
        .and_then(|()| stream.flush())
        .map_err(Unheard::Lost)?;
    stream.set_nonblocking(true).map_err(Unheard::Lost)?;

    let asked = Instant::now();
    let mut answer = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    loop {
        if interrupted.load(Ordering::SeqCst) {
            let _ = stream.shutdown(Shutdown::Both);
            return Err(Unheard::Interrupted);
        }
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                answer.extend_from_slice(&buffer[..read]);
                // Done at the last byte its Content-Length counts, not at
                // the close: on Windows a server's close is often a reset,
                // which would lose an answer already here.
                if whole(&answer) {
                    break;
                }
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                if patience.is_some_and(|patience| asked.elapsed() > patience) {
                    return Err(Unheard::Lost(io::Error::from(ErrorKind::TimedOut)));
                }
                std::thread::sleep(TICK);
            }
            Err(error) => return Err(Unheard::Lost(error)),
        }
    }
    body_of(&answer)
}

/// Whether `answer` holds a whole HTTP answer: its head, and as many bytes
/// of body as the head's Content-Length says. Without a Content-Length an
/// answer ends where the connection does.
fn whole(answer: &[u8]) -> bool {
    let Some(end) = answer.windows(4).position(|four| four == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&answer[..end]);
    let length = head.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    });
    length.is_some_and(|length| answer.len() >= end + 4 + length)
}

/// The JSON body of a whole HTTP answer, when its status is 200.
fn body_of(answer: &[u8]) -> Result<Value, Unheard> {
    let text = String::from_utf8_lossy(answer);
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return Err(Unheard::Lost(io::Error::from(ErrorKind::UnexpectedEof)));
    };
    let status = head.lines().next().unwrap_or_default();
    if !status.starts_with("HTTP/1.1 200") {
        return Err(Unheard::Garbled(status.to_owned()));
    }
    serde_json::from_str(body).map_err(|error| Unheard::Garbled(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_whole_answer_with_status_200_is_read() {
        let answer =
            b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\n{\"result\":1}";
        assert_eq!(body_of(answer).expect("an answer")["result"], 1);
        assert!(matches!(
            body_of(b"HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n"),
            Err(Unheard::Garbled(status)) if status.contains("413")
        ));
        assert!(matches!(
            body_of(b"HTTP/1.1 200 OK\r\nContent-Le"),
            Err(Unheard::Lost(_))
        ));
        assert!(matches!(
            body_of(b"HTTP/1.1 200 OK\r\n\r\nnot json"),
            Err(Unheard::Garbled(_))
        ));
    }

    /// A server that reads the whole request, then waits `delay` before it
    /// answers in two pieces — or never answers when `delay` is `None`.
    fn slow_server(delay: Option<Duration>) -> SocketAddr {
        use std::io::{BufRead, BufReader};
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("a port");
        let address = listener.local_addr().expect("an address");
        std::thread::spawn(move || {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut writer = stream.try_clone().expect("a writer");
            let mut reader = BufReader::new(stream);
            let mut length = 0;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim_end().is_empty() {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);
            let Some(delay) = delay else {
                std::thread::sleep(Duration::from_secs(5));
                return;
            };
            std::thread::sleep(delay);
            let reply = r#"{"jsonrpc":"2.0","id":1,"result":{"answered":true}}"#;
            let _ = writer.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    reply.len()
                )
                .as_bytes(),
            );
            std::thread::sleep(TICK * 3);
            let _ = writer.write_all(reply.as_bytes());
        });
        address
    }

    /// The answer of a slow server is read whole, however many looks at the
    /// socket it took — the road a long rewrite takes. On Windows the wait
    /// used to be a read deadline per look, after which the connection is
    /// "in an indeterminate state" (SO_RCVTIMEO), and the CLI now and then
    /// said the application had stopped answering.
    #[test]
    fn an_answer_that_takes_many_looks_is_read_whole() {
        let address = slow_server(Some(TICK * 15));
        let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call"});
        let answer = exchange(
            address,
            &request,
            Some(Duration::from_secs(10)),
            &AtomicBool::new(false),
        )
        .expect("the whole answer");
        assert_eq!(answer["result"]["answered"], true);
    }

    /// A whole answer is the answer, however the server then hangs up: a
    /// reset after the last byte of its body (what Windows sends when a
    /// socket is closed with anything left unread — found on the Windows
    /// runner and on Windows 10, `ConnectionReset` after the 994 bytes of a
    /// whole answer) loses nothing. The server here leaves the request's body
    /// unread, which makes its close a reset on Linux too.
    #[test]
    fn a_whole_answer_survives_a_reset_after_it() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("a port");
        let address = listener.local_addr().expect("an address");
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            // The head, a byte at a time, so the body stays in the socket.
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                head.push(byte[0]);
            }
            let reply = r#"{"jsonrpc":"2.0","id":1,"result":{"answered":true}}"#;
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                )
                .as_bytes(),
            );
            std::thread::sleep(TICK * 10);
            // Dropped with the body unread: a reset.
        });
        let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"padding": "x".repeat(4096)}});
        let answer = exchange(
            address,
            &request,
            Some(Duration::from_secs(10)),
            &AtomicBool::new(false),
        )
        .expect("the whole answer, the reset after it notwithstanding");
        assert_eq!(answer["result"]["answered"], true);
    }

    /// Patience still runs out, and Ctrl-C still stops the wait.
    #[test]
    fn a_wait_ends_on_patience_and_on_ctrl_c() {
        let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call"});
        let lost = exchange(
            slow_server(None),
            &request,
            Some(TICK * 5),
            &AtomicBool::new(false),
        );
        assert!(
            matches!(&lost, Err(Unheard::Lost(error)) if error.kind() == ErrorKind::TimedOut),
            "{lost:?}"
        );
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let stopper = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(TICK * 5);
            stopper.store(true, Ordering::SeqCst);
        });
        let stopped = exchange(slow_server(None), &request, None, &stop);
        assert!(matches!(stopped, Err(Unheard::Interrupted)), "{stopped:?}");
    }
}
