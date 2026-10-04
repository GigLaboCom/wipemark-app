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

/// How long one wait for the answer is, between two looks at Ctrl-C.
const TICK: Duration = Duration::from_millis(200);

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
/// `interrupted` hangs up.
pub(crate) fn rewrite(
    app: App,
    arguments: &Value,
    interrupted: &AtomicBool,
) -> Result<Value, Unheard> {
    let call = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "rewrite", "arguments": arguments },
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
    stream.set_read_timeout(Some(TICK)).map_err(Unheard::Lost)?;

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
            Ok(read) => answer.extend_from_slice(&buffer[..read]),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) =>
            {
                if patience.is_some_and(|patience| asked.elapsed() > patience) {
                    return Err(Unheard::Lost(io::Error::from(ErrorKind::TimedOut)));
                }
            }
            Err(error) => return Err(Unheard::Lost(error)),
        }
    }
    body_of(&answer)
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
}
