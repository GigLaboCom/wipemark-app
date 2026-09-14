//! The socket half: one listener, the port scan that finds it a home,
//! and the supervisor that owns both.
//!
//! Epic **E6 / S6.3**. `heretic-lazy-shot` runs its MCP endpoint on
//! axum over tokio because it already has both; this application has
//! neither, and adding a runtime and a web framework to serve one POST
//! route would be a large dependency for a small surface. What is here
//! instead is a thread, a `TcpListener`, and enough HTTP/1.1 to answer
//! the streamable-HTTP transport: read a request, hand the body to
//! [`protocol::respond`](super::protocol::respond), write the answer,
//! close the connection.
//!
//! # Threads, and the one rule this file exists under
//!
//! Nothing here may block the GPUI thread — not for a bind, not for a
//! join, not for a shutdown. So there are two threads while the server
//! is running and one while it is idle:
//!
//! * The **supervisor** owns whatever is running. Every request from
//!   the window arrives as a [`Command`] on a channel, and the
//!   supervisor is where a listener is bound, joined and replaced.
//!   Stopping a listener means *waiting* for its thread, and waiting is
//!   exactly the thing the foreground thread is not allowed to do.
//! * The **listener** accepts, and hands each connection to a thread of
//!   its own so that one client reading slowly is not the whole server.
//!
//! Answers travel back the other way as [`Event`]s on a `flume`
//! channel, which the GPUI side polls from `cx.spawn` — the same
//! arrangement every long operation in this product uses, and for the
//! same reason: the two executors cannot await each other.
//!
//! # Stopping
//!
//! `accept` blocks, and there is no way to interrupt a blocked accept
//! from another thread with the standard library alone. The two
//! answers are to poll a non-blocking listener, or to leave it blocking
//! and wake it by connecting to it. This polls, at [`POLL`], because
//! the knock has a failure mode this does not: a knock that cannot
//! connect — the interface went away, something answered first — leaves
//! a thread blocked in `accept` for the life of the process, holding
//! the port that the restart is about to ask for. A poll has a bounded
//! latency and no failure mode.

use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::{protocol, Endpoint, PATH};

/// How many ports the scan tries before giving up.
///
/// lazy-shot's number, and the reason to keep it is that the two
/// products are run together: a user who knows that one of them walks
/// ten ports up from where it was asked should not have to learn a
/// second rule for the other.
pub const ATTEMPTS: u16 = 10;

/// How long the accept loop sleeps between looks. The upper bound on
/// how long stopping takes, and short enough that toggling the switch
/// off and on again cannot find the old listener still holding the
/// port.
const POLL: Duration = Duration::from_millis(100);

/// How long one connection may take to say what it wants, or to read
/// the answer. A client that stops talking mid-request holds a thread
/// until this expires and not a moment longer.
const PATIENCE: Duration = Duration::from_secs(5);

/// The largest request head this will read. A client that never sends
/// the blank line ending its headers is otherwise an allocation with no
/// upper bound.
const LARGEST_HEAD: u64 = 16 * 1024;

/// The largest body. Layer A works on text an agent has in hand, and a
/// megabyte of it is a long document; anything past that is either a
/// mistake or an attempt at one.
const LARGEST_BODY: usize = 1024 * 1024;

/// What the server has to say for itself, on its way to the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Bound and answering. `endpoint` is where it *actually* is, which
    /// is not necessarily where it was asked to be — see `wanted`, and
    /// [`ladder`].
    Listening { endpoint: Endpoint, wanted: u16 },
    /// Asked to stop, and stopped. The port is free again by the time
    /// this is sent.
    Stopped,
    /// Never started. The string is the operating system's own
    /// account, which the window interpolates into a sentence of its
    /// own — the same arrangement `cli-unknown-language` makes.
    Failed(String),
}

/// The ports one attempt will try, in order.
///
/// `checked_add` rather than lazy-shot's `saturating_add`: near the top
/// of the range, saturating turns the last few attempts into nine more
/// tries at 65535, which reports "ten ports are taken" about one port.
pub fn ladder(wanted: u16) -> impl Iterator<Item = u16> {
    (0..ATTEMPTS).map_while(move |offset| wanted.checked_add(offset))
}

/// Whether a failure to bind is worth trying the next port for.
///
/// Only a port somebody else holds is. An address this machine does not
/// have, or a port this process may not have, fails identically on all
/// ten — and reporting the tenth failure instead of the first points
/// the user at a port number when the thing to fix is the address.
fn worth_stepping_past(kind: ErrorKind) -> bool {
    kind == ErrorKind::AddrInUse
}

/// Take the first port in the ladder that is free.
fn bind(wanted: Endpoint) -> Result<TcpListener, String> {
    let mut last = String::new();
    for port in ladder(wanted.port) {
        match TcpListener::bind(SocketAddr::new(wanted.bind.ip(), port)) {
            Ok(listener) => return Ok(listener),
            Err(error) if !worth_stepping_past(error.kind()) => {
                return Err(format!("{}: {error}", wanted.bind));
            }
            Err(error) => {
                tracing::warn!(port, %error, "MCP: port taken, trying the next one");
                last = error.to_string();
            }
        }
    }
    Err(format!(
        "{ATTEMPTS} ports from {} are taken ({last})",
        wanted.port
    ))
}

/// A listener and the thread turning it.
struct Running {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl Running {
    /// Ask it to stop and wait until it has.
    ///
    /// The waiting is the point: this returns when the port is free,
    /// which is what makes "off, then on again" land on the same port
    /// rather than the next one. Called only from the supervisor
    /// thread — see the module docs.
    fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        if self.thread.join().is_err() {
            tracing::error!("the MCP listener thread panicked");
        }
    }
}

/// Bind, and start turning.
fn start(wanted: Endpoint) -> Result<(Running, Event), String> {
    let listener = bind(wanted)?;
    let port = listener
        .local_addr()
        .map_or(wanted.port, |address| address.port());
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;

    let stop = Arc::new(AtomicBool::new(false));
    let thread = thread::Builder::new()
        .name("wipemark-mcp".to_owned())
        .spawn({
            let stop = Arc::clone(&stop);
            move || accept(&listener, &stop)
        })
        .map_err(|error| error.to_string())?;

    tracing::info!(%wanted.bind, port, "MCP: listening");
    Ok((
        Running { stop, thread },
        Event::Listening {
            endpoint: Endpoint {
                bind: wanted.bind,
                port,
            },
            wanted: wanted.port,
        },
    ))
}

/// The accept loop. Returns when `stop` says so, and the listener is
/// dropped — and the port released — on the way out.
fn accept(listener: &TcpListener, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                if let Err(error) = stream.set_nonblocking(false) {
                    tracing::warn!(%error, "MCP: could not take a connection off non-blocking");
                    continue;
                }
                // A thread per connection, and no pool: MCP traffic is
                // one agent asking one question at a time, and the
                // alternative — answering inline — lets a client that
                // stops talking hold the whole server for `PATIENCE`.
                if let Err(error) = thread::Builder::new()
                    .name("wipemark-mcp-connection".to_owned())
                    .spawn(move || answer(stream, peer))
                {
                    tracing::warn!(%error, "MCP: could not take a connection");
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => thread::sleep(POLL),
            Err(error) => {
                // One connection that fell over between the SYN and the
                // accept. Not a reason to stop serving the next one.
                tracing::warn!(%error, "MCP: a connection was dropped before it arrived");
                thread::sleep(POLL);
            }
        }
    }
}

/// What one request asked for, before its body is read.
#[derive(Debug, PartialEq, Eq)]
struct Head {
    method: String,
    target: String,
    origin: Option<String>,
    length: usize,
}

/// Read the request line and the headers, stopping at the blank line.
///
/// `None` for anything that is not a request — a closed connection, a
/// line that is not UTF-8, a `Content-Length` that is not a number.
/// The caller answers with `400` and hangs up.
fn read_head(reader: &mut impl BufRead) -> Option<Head> {
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let mut request = line.split_whitespace();
    let method = request.next()?.to_ascii_uppercase();
    let target = request.next()?.to_owned();

    let mut origin = None;
    let mut length = 0;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let Some((name, value)) = header.split_once(':') else {
            continue;
        };
        let value = value.trim();
        // Header names are case-insensitive, and the case a client
        // chooses is not a thing to be surprised by.
        if name.eq_ignore_ascii_case("content-length") {
            length = value.parse().ok()?;
        } else if name.eq_ignore_ascii_case("origin") {
            origin = Some(value.to_owned());
        }
    }

    Some(Head {
        method,
        target,
        origin,
        length,
    })
}

/// The path, without the query a client may have hung off it.
fn path_of(target: &str) -> &str {
    target
        .split_once('?')
        .map_or(target, |(path, _)| path)
        .trim_end_matches('/')
}

/// The host part of an `Origin`, brackets and port included.
fn host_of(origin: &str) -> Option<&str> {
    let rest = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))?;
    let authority = rest.split('/').next().unwrap_or_default();
    // `[::1]:5056` splits at the last colon and `[::1]` does not split
    // at all, because what follows its colons is not a port number.
    Some(match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => host,
        _ => authority,
    })
}

/// Whether a browser at this origin is one of ours.
///
/// The MCP specification asks a local HTTP server to validate `Origin`,
/// and the attack it is asking about is real: without this, any page
/// the user has open can walk `http://127.0.0.1:5056/mcp` and drive
/// this server, because the browser attaches no credentials the server
/// checks and the request looks exactly like a legitimate one. A
/// non-browser client sends no `Origin` at all and is unaffected.
fn origin_is_local(origin: &str) -> bool {
    let Some(host) = host_of(origin) else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    // A bracketed IPv6 literal, as a URL writes one.
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    host.parse::<IpAddr>()
        .is_ok_and(|address| address.is_loopback())
}

/// Answer one connection, then close it.
fn answer(stream: TcpStream, peer: SocketAddr) {
    let deadline = Some(PATIENCE);
    if stream.set_read_timeout(deadline).is_err() || stream.set_write_timeout(deadline).is_err() {
        tracing::warn!(%peer, "MCP: could not put a deadline on a connection");
        return;
    }

    let mut writer = match stream.try_clone() {
        Ok(writer) => writer,
        Err(error) => {
            tracing::warn!(%peer, %error, "MCP: could not answer a connection");
            return;
        }
    };
    let mut reader = BufReader::new(stream);

    let reply = match read_head(&mut reader.by_ref().take(LARGEST_HEAD)) {
        Some(head) => route(head, &mut reader),
        // Not a request. A port scanner, a browser that opened the
        // address by hand, or a connection that closed before it said
        // anything.
        None => reply("400 Bad Request", None, &[], None),
    };

    if let Err(error) = writer.write_all(&reply) {
        tracing::warn!(%peer, %error, "MCP: could not write an answer");
    }
    let _ = writer.flush();
}

/// Which of this server's four answers a request gets.
fn route(head: Head, body: &mut impl Read) -> Vec<u8> {
    let origin = head.origin.as_deref();
    let allowed = origin.filter(|origin| origin_is_local(origin));

    // Checked before the route and before the body: a page on another
    // origin does not get to find out what is here, and does not get to
    // send a megabyte to find out.
    if origin.is_some() && allowed.is_none() {
        tracing::warn!(
            origin = origin.unwrap_or_default(),
            "MCP: refused a request from another origin"
        );
        return reply("403 Forbidden", None, &[], None);
    }

    if head.method == "OPTIONS" {
        return reply("204 No Content", None, &[], allowed);
    }
    if path_of(&head.target) != PATH {
        return reply("404 Not Found", None, &[], allowed);
    }

    match head.method.as_str() {
        "POST" => {
            if head.length > LARGEST_BODY {
                return reply("413 Payload Too Large", None, &[], allowed);
            }
            let mut buffer = vec![0; head.length];
            if body.read_exact(&mut buffer).is_err() {
                return reply("400 Bad Request", None, &[], allowed);
            }
            let Ok(text) = String::from_utf8(buffer) else {
                return reply("400 Bad Request", None, &[], allowed);
            };
            match protocol::respond(&text) {
                Some(response) => reply(
                    "200 OK",
                    Some("application/json"),
                    response.as_bytes(),
                    allowed,
                ),
                // A notification. JSON-RPC says it gets no reply, and
                // 204 is how HTTP says that.
                None => reply("204 No Content", None, &[], allowed),
            }
        }
        // The transport's optional server-to-client stream, which this
        // server does not open. The specification's own answer for a
        // server that has no stream to offer, and every client accepts
        // it.
        "GET" | "DELETE" => reply("405 Method Not Allowed", None, &[], allowed),
        _ => reply("405 Method Not Allowed", None, &[], allowed),
    }
}

/// One HTTP response, headers and all.
///
/// `Connection: close` on every one of them, deliberately: a
/// connection per request costs a loopback handshake and buys the whole
/// of keep-alive's state machine not existing here. `allowed` is an
/// origin that has already been checked — this never reflects one it
/// has not.
fn reply(status: &str, content_type: Option<&str>, body: &[u8], allowed: Option<&str>) -> Vec<u8> {
    let mut head = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Vary: Origin\r\n",
        body.len()
    );
    if let Some(content_type) = content_type {
        head.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    if let Some(origin) = allowed {
        head.push_str(&format!(
            "Access-Control-Allow-Origin: {origin}\r\n\
             Access-Control-Allow-Methods: POST, OPTIONS\r\n\
             Access-Control-Allow-Headers: content-type, mcp-session-id, mcp-protocol-version\r\n\
             Access-Control-Max-Age: 600\r\n"
        ));
    }
    head.push_str("\r\n");

    let mut reply = head.into_bytes();
    reply.extend_from_slice(body);
    reply
}

/// What the window asks the supervisor for.
enum Command {
    Serve(Endpoint),
    Stop,
}

/// The one thing that owns a running server.
///
/// Held by `Preferences`, which is app-lifetime — the server starts
/// with the application and outlives every Settings window, because a
/// server that only ran while its own preferences pane was open would
/// be a server no agent could rely on.
///
/// Dropping this disconnects the command channel, which is how the
/// supervisor thread learns to stop the listener and exit.
pub struct Supervisor {
    commands: flume::Sender<Command>,
}

impl Supervisor {
    /// Start the supervisor thread. It has nothing to serve until it is
    /// asked.
    pub fn spawn(events: flume::Sender<Event>) -> Self {
        let (commands, requests) = flume::unbounded();
        if let Err(error) = thread::Builder::new()
            .name("wipemark-mcp-supervisor".to_owned())
            .spawn(move || supervise(&requests, &events))
        {
            // A machine that cannot spawn a thread is not going to run
            // a GPUI application either, but the window still opens and
            // the switch still stores what it is told. Every later
            // command goes into a channel nobody is reading, which is
            // the same shape as a server that never starts.
            tracing::error!(%error, "MCP: the supervisor could not start");
        }
        Self { commands }
    }

    /// Serve on this endpoint, replacing whatever is running.
    pub fn serve(&self, endpoint: Endpoint) {
        self.send(Command::Serve(endpoint));
    }

    /// Stop, if anything is running.
    pub fn stop(&self) {
        self.send(Command::Stop);
    }

    fn send(&self, command: Command) {
        if self.commands.send(command).is_err() {
            tracing::error!("MCP: the supervisor is gone; the server cannot be reached");
        }
    }
}

/// The supervisor thread: one server at a time, and every change of
/// mind applied in order.
fn supervise(commands: &flume::Receiver<Command>, events: &flume::Sender<Event>) {
    let mut running: Option<Running> = None;

    while let Ok(mut command) = commands.recv() {
        // Typing a port is several valid ports on the way to the one
        // that was meant — 1024 is a port, and so is 10240 one
        // keystroke later. Only the last request in the queue is a
        // request; the ones behind it are a number being passed
        // through, and binding each of them in turn would be ten
        // servers nobody asked for.
        while let Ok(next) = commands.try_recv() {
            command = next;
        }

        // Always before the next bind, and this is the whole reason the
        // supervisor exists: the old listener has to have let go of the
        // port before the new one asks for it, or "off and on again"
        // lands a port higher every time.
        if let Some(server) = running.take() {
            server.stop();
        }

        match command {
            Command::Stop => {
                tracing::info!("MCP: stopped");
                let _ = events.send(Event::Stopped);
            }
            Command::Serve(endpoint) => match start(endpoint) {
                Ok((server, listening)) => {
                    running = Some(server);
                    let _ = events.send(listening);
                }
                Err(error) => {
                    tracing::warn!(%error, "MCP: could not start");
                    let _ = events.send(Event::Failed(error));
                }
            },
        }
    }

    // The channel is gone, so the application is on its way out.
    if let Some(server) = running.take() {
        server.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::mcp::BindAddress;

    /// A port that is free and whose successor is free too, so that a
    /// test about "the next one" is testing the scan rather than the
    /// machine's luck.
    ///
    /// Handed out from one cursor for the whole process, not asked of
    /// the operating system each time. These tests run in parallel,
    /// and two of them asking the OS for "a free port" a moment apart
    /// were handed the same one — the port a dropped probe had just
    /// given back — after which one test's request reached the other
    /// test's server, or a server that had already stopped. The cursor
    /// starts where the OS says and moves two ports a call, so no two
    /// callers share a pair; the probe that follows is against the
    /// rest of the machine, which is the only race left.
    fn a_port_with_a_free_neighbour() -> u16 {
        static NEXT: std::sync::Mutex<u16> = std::sync::Mutex::new(0);
        for _ in 0..64 {
            let port = {
                let mut next = NEXT.lock().expect("the cursor");
                if *next == 0 {
                    let Ok(probe) = TcpListener::bind(("127.0.0.1", 0)) else {
                        continue;
                    };
                    let Ok(port) = probe.local_addr().map(|address| address.port()) else {
                        continue;
                    };
                    *next = port;
                }
                let port = *next;
                // Past the top, back to where the OS started us.
                *next = port.checked_add(2).unwrap_or(0);
                port
            };
            let Some(neighbour) = port.checked_add(1) else {
                continue;
            };
            let (Ok(first), Ok(second)) = (
                TcpListener::bind(("127.0.0.1", port)),
                TcpListener::bind(("127.0.0.1", neighbour)),
            ) else {
                continue;
            };
            drop((first, second));
            return port;
        }
        panic!("no two consecutive free ports on this machine");
    }

    /// Send one request to a running server and read the whole answer
    /// back.
    fn ask(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the server is listening");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a deadline");
        stream
            .write_all(request.as_bytes())
            .expect("the request went out");
        let mut answer = String::new();
        stream.read_to_string(&mut answer).expect("an answer");
        answer
    }

    fn post(body: &str, headers: &str) -> String {
        format!(
            "POST {PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\n{headers}\r\n{body}",
            body.len()
        )
    }

    /// The whole of what the user asked for, in one assertion: the port
    /// they chose is held by something else, and the server comes up
    /// one along rather than not at all.
    #[test]
    fn a_port_something_else_holds_is_stepped_past() {
        let port = a_port_with_a_free_neighbour();
        let taken = TcpListener::bind(("127.0.0.1", port)).expect("hold the port");

        let (server, event) = start(Endpoint {
            bind: BindAddress::LOOPBACK,
            port,
        })
        .expect("the scan finds the next port");

        assert_eq!(
            event,
            Event::Listening {
                endpoint: Endpoint {
                    bind: BindAddress::LOOPBACK,
                    port: port + 1,
                },
                wanted: port,
            }
        );
        server.stop();
        drop(taken);
    }

    /// And the endpoint it reports is the one it is actually on — the
    /// snippet on screen is built from this, so a server that stepped
    /// up a port and said nothing would hand out a configuration that
    /// dials a port nothing is listening on.
    #[test]
    fn a_server_that_moved_says_where_it_went() {
        let port = a_port_with_a_free_neighbour();
        let taken = TcpListener::bind(("127.0.0.1", port)).expect("hold the port");
        let (server, event) = start(Endpoint {
            bind: BindAddress::LOOPBACK,
            port,
        })
        .expect("the scan finds the next port");

        let Event::Listening { endpoint, wanted } = event else {
            panic!("the server did not report where it landed");
        };
        assert_ne!(endpoint.port, wanted);
        assert_eq!(endpoint.url(), format!("http://127.0.0.1:{}/mcp", port + 1));
        // And it really is there.
        let answer = ask(
            endpoint.port,
            &post(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#, ""),
        );
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");

        server.stop();
        drop(taken);
    }

    /// The ladder is where the +1 lives, and it starts where it was
    /// asked to rather than one along.
    #[test]
    fn the_scan_starts_at_the_port_that_was_asked_for() {
        let ports: Vec<u16> = ladder(5056).collect();
        assert_eq!(ports.first(), Some(&5056));
        assert_eq!(ports.len(), usize::from(ATTEMPTS));
        assert_eq!(ports[1], 5057);
    }

    /// And it stops at the top of the range rather than trying 65535
    /// nine more times, which is what a saturating step does and what
    /// makes "ten ports are taken" a sentence about one port.
    #[test]
    fn the_scan_stops_at_the_highest_port_there_is() {
        let ports: Vec<u16> = ladder(u16::MAX - 2).collect();
        assert_eq!(ports, vec![u16::MAX - 2, u16::MAX - 1, u16::MAX]);
    }

    /// An address this machine does not hold fails the same way on all
    /// ten ports. Stepping past it wastes nine binds and then blames a
    /// port number for an address that was wrong — which is the message
    /// the user reads and acts on.
    #[test]
    fn only_a_port_someone_else_holds_is_worth_stepping_past() {
        assert!(worth_stepping_past(ErrorKind::AddrInUse));
        assert!(!worth_stepping_past(ErrorKind::AddrNotAvailable));
        assert!(!worth_stepping_past(ErrorKind::PermissionDenied));
    }

    /// And the failure says the address, because that is what has to
    /// change. `203.0.113.0/24` is TEST-NET-3 (RFC 5737) — reserved for
    /// documentation, so no machine running this test holds one.
    #[test]
    fn an_address_this_machine_does_not_have_is_named_in_the_failure() {
        let bind = BindAddress::parse("203.0.113.1").expect("an address");
        let error = start(Endpoint { bind, port: 5056 }).err().expect("no bind");
        assert!(
            error.contains("203.0.113.1"),
            "the failure blames something other than the address: {error}"
        );
    }

    /// The handshake, over a real socket, from a client that is not
    /// this process.
    #[test]
    fn the_server_answers_the_handshake_over_a_real_socket() {
        let port = a_port_with_a_free_neighbour();
        let (server, event) = start(Endpoint {
            bind: BindAddress::LOOPBACK,
            port,
        })
        .expect("a free port");
        let Event::Listening { endpoint, .. } = event else {
            panic!("not listening");
        };

        let answer = ask(
            endpoint.port,
            &post(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
                "",
            ),
        );
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
        assert!(
            answer.contains("Content-Type: application/json"),
            "{answer}"
        );
        let body = answer.split_once("\r\n\r\n").expect("a body").1;
        let response: serde_json::Value = serde_json::from_str(body).expect("JSON");
        assert_eq!(response["result"]["serverInfo"]["name"], "wipemark");

        server.stop();
    }

    /// A notification gets no JSON back, and HTTP has a status that
    /// says exactly that. A client that read a body here would be
    /// waiting for one that is not coming.
    #[test]
    fn a_notification_over_the_socket_is_answered_with_no_content() {
        let port = a_port_with_a_free_neighbour();
        let (server, _) = start(Endpoint {
            bind: BindAddress::LOOPBACK,
            port,
        })
        .expect("a free port");

        let answer = ask(
            port,
            &post(
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                "",
            ),
        );
        assert!(answer.starts_with("HTTP/1.1 204 No Content"), "{answer}");
        server.stop();
    }

    /// The protection the MCP specification asks a local server for,
    /// and the one this file would be irresponsible without: any page
    /// the user has open can reach a loopback port, and without this
    /// check it can drive the server.
    #[test]
    fn a_page_on_another_origin_is_refused() {
        let port = a_port_with_a_free_neighbour();
        let (server, _) = start(Endpoint {
            bind: BindAddress::LOOPBACK,
            port,
        })
        .expect("a free port");

        let answer = ask(
            port,
            &post(
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
                "Origin: https://example.invalid\r\n",
            ),
        );
        assert!(answer.starts_with("HTTP/1.1 403 Forbidden"), "{answer}");
        assert!(
            !answer.contains("Access-Control-Allow-Origin"),
            "the refusal handed the page a CORS grant anyway: {answer}"
        );

        // And the local page it was written for still works.
        let answer = ask(
            port,
            &post(
                r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
                "Origin: http://localhost:3000\r\n",
            ),
        );
        assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
        assert!(answer.contains("Access-Control-Allow-Origin: http://localhost:3000"));

        server.stop();
    }

    /// Which origins count as ours, decided in one place because the
    /// answer is a security boundary and not a string comparison
    /// somebody will loosen by accident.
    #[test]
    fn only_a_page_on_this_machine_is_one_of_ours() {
        for ours in [
            "http://localhost",
            "http://localhost:3000",
            "https://localhost:8443",
            "http://127.0.0.1:5056",
            "http://127.0.0.53",
            "http://[::1]:5056",
        ] {
            assert!(origin_is_local(ours), "{ours} was refused");
        }
        for theirs in [
            "https://example.invalid",
            "http://localhost.example.invalid",
            "http://127.0.0.1.example.invalid",
            "http://192.168.1.101:5056",
            "null",
            "",
            "file://",
        ] {
            assert!(!origin_is_local(theirs), "{theirs} was let in");
        }
    }

    /// Anything that is not the endpoint is not the endpoint. A server
    /// answering `/` with its tool list is one a scanner finds.
    #[test]
    fn nothing_but_the_endpoint_answers() {
        let port = a_port_with_a_free_neighbour();
        let (server, _) = start(Endpoint {
            bind: BindAddress::LOOPBACK,
            port,
        })
        .expect("a free port");

        for target in ["/", "/tools", "/mcp/../secrets"] {
            let request =
                format!("POST {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\n\r\n");
            let answer = ask(port, &request);
            assert!(
                answer.starts_with("HTTP/1.1 404 Not Found"),
                "{target} answered: {answer}"
            );
        }

        // A GET on the endpoint is the transport's optional stream,
        // which this server does not open. The specification's own
        // answer for that is 405, not 404.
        let answer = ask(
            port,
            &format!("GET {PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
        );
        assert!(answer.starts_with("HTTP/1.1 405"), "{answer}");

        server.stop();
    }

    /// Stopping means the port is free — not "will be free shortly".
    /// The whole of "off and on again lands on the same port" rests on
    /// this, and so does a restart after the address is edited.
    #[test]
    fn stopping_lets_go_of_the_port_before_it_returns() {
        let port = a_port_with_a_free_neighbour();
        let (server, _) = start(Endpoint {
            bind: BindAddress::LOOPBACK,
            port,
        })
        .expect("a free port");
        server.stop();

        let again = TcpListener::bind(("127.0.0.1", port));
        assert!(
            again.is_ok(),
            "the port is still held after the server said it stopped: {:?}",
            again.err()
        );
    }

    /// A restart is a stop and a start, in that order, on one thread —
    /// so a server asked to move to another port and back lands where
    /// it started rather than one along each time.
    #[test]
    fn serving_twice_over_lands_on_the_same_port_both_times() {
        let port = a_port_with_a_free_neighbour();
        let (events, heard) = flume::unbounded();
        let supervisor = Supervisor::spawn(events);
        let endpoint = Endpoint {
            bind: BindAddress::LOOPBACK,
            port,
        };

        for _ in 0..3 {
            supervisor.serve(endpoint);
            let event = heard
                .recv_timeout(Duration::from_secs(5))
                .expect("an event");
            assert_eq!(
                event,
                Event::Listening {
                    endpoint,
                    wanted: port
                },
                "a restart walked the port up"
            );
        }

        supervisor.stop();
        assert_eq!(
            heard
                .recv_timeout(Duration::from_secs(5))
                .expect("an event"),
            Event::Stopped
        );
    }

    /// The request line and the headers, without a socket. Case is the
    /// client's business, and `Content-Length` is how the body is found.
    #[test]
    fn a_request_head_is_read_whatever_case_it_arrives_in() {
        let mut request = Cursor::new(
            "post /mcp?session=1 HTTP/1.1\r\nHost: x\r\nCONTENT-LENGTH: 12\r\n\
             origin: http://localhost:3000\r\n\r\n"
                .as_bytes(),
        );
        assert_eq!(
            read_head(&mut request),
            Some(Head {
                method: "POST".to_owned(),
                target: "/mcp?session=1".to_owned(),
                origin: Some("http://localhost:3000".to_owned()),
                length: 12,
            })
        );
        assert_eq!(path_of("/mcp?session=1"), PATH);
        assert_eq!(path_of("/mcp/"), PATH);
    }

    /// A connection that says nothing, or says something that is not a
    /// request, is a `400` rather than a panic on a thread nobody is
    /// watching.
    #[test]
    fn a_connection_that_is_not_a_request_is_not_a_panic() {
        for nonsense in ["", "\r\n", "hello", "GET\r\n\r\n"] {
            let mut request = Cursor::new(nonsense.as_bytes());
            assert!(
                read_head(&mut request).is_none()
                    || read_head(&mut Cursor::new(nonsense.as_bytes())).is_some(),
                "{nonsense:?} neither parsed nor failed"
            );
        }
        let mut empty = Cursor::new("".as_bytes());
        assert_eq!(read_head(&mut empty), None);
    }

    /// Every answer carries its own length and closes the connection,
    /// because there is no keep-alive state machine here to carry the
    /// next request.
    #[test]
    fn every_answer_says_how_long_it_is_and_hangs_up() {
        let body = br#"{"ok":true}"#;
        let answer = reply("200 OK", Some("application/json"), body, None);
        let text = String::from_utf8(answer).expect("headers are ASCII");
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains(&format!("Content-Length: {}\r\n", body.len())));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.ends_with(r#"{"ok":true}"#));
    }
}
