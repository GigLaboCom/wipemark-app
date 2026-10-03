//! The transport: one request, its retries, and the streamed answer read
//! to its end marker — blocking, on the request's own thread (D58).
//!
//! # The rules, and where each one is enforced
//!
//! * **`http` and `https` only.** [`checked`] refuses any other scheme, and
//!   an authority carrying userinfo, before a socket is opened. The
//!   application's `engine::BaseUrl` already refuses both; this is defence
//!   in depth, not a second rule.
//! * **No redirect is followed.** The agent has `max_redirects(0)`, so
//!   ureq hands a 3xx back as it is, and a 3xx is
//!   [`Unavailable::Redirected`] naming the status and the *origin* the
//!   server pointed at — never its path or query. A client that followed
//!   would carry the `Authorization` header to a host nobody validated
//!   (`docs/sdd/layer-b-rewrite-reference.md` §2).
//! * **Retries only before the first byte of an answer.** A connection that
//!   could not be made, a 429 and a 502/503/504 are tried again, at most
//!   twice, waiting what `Retry-After` asks up to ten seconds. Nothing is
//!   retried once a 2xx has started to arrive: the half that arrived has
//!   already been streamed to somebody.
//! * **A read timeout, not a total one.** `timeout` is how long the server
//!   may stay silent — before its headers or between two pieces of the
//!   answer — and a long generation that keeps talking is never cut off.
//!   ureq has only a total budget for a body, so [`IdleRead`] wraps the
//!   connection and caps every wait on the socket instead.
//! * **No gzip** (the workspace builds ureq without it), so the byte counts
//!   in the log are what crossed the wire.
//!
//! Nothing here logs a prompt, a completion, a header value or a key:
//! status codes, byte counts, timings and the origin.

use std::io::Read;
use std::time::{Duration, Instant};

use ureq::unversioned::resolver::DefaultResolver;
use ureq::unversioned::transport::time::Duration as UreqDuration;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, DefaultConnector, NextTimeout, Transport,
};
use wipemark_secret::Secret;

use super::sse::{ended_early, Assembled, Format, Lines};
use crate::{Completion, EngineError, Unavailable};

/// How long a connection may take to open, TLS handshake included.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How many times a request is tried again before its first byte.
pub(crate) const RETRIES: usize = 2;

/// The longest `Retry-After` honoured.
pub(crate) const LONGEST_WAIT: Duration = Duration::from_secs(10);

/// The waits between attempts when the server names none.
const BACKOFF: [Duration; RETRIES] = [Duration::from_millis(500), Duration::from_millis(1000)];

/// How much of an error's body is kept as its detail.
const DETAIL_BYTES: u64 = 2048;

/// The agent one engine sends with.
///
/// The platform's trust roots and proxy settings, as the downloader's agent
/// has them; nothing added.
pub(crate) fn agent(read_timeout: Duration) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .user_agent(concat!("wipemark/", env!("CARGO_PKG_VERSION")))
        .build();
    ureq::Agent::with_parts(
        config,
        DefaultConnector::default().chain(IdleRead(read_timeout)),
        DefaultResolver::default(),
    )
}

/// Caps every wait for input on a connection at the read timeout.
///
/// `ureq::unversioned` is the transport API ureq does not promise to keep
/// stable across minor versions; `Cargo.lock` pins the version this was
/// written against, and a change there is a compile error here rather than
/// a silent one.
#[derive(Debug)]
struct IdleRead(Duration);

impl Connector<Box<dyn Transport>> for IdleRead {
    type Out = Idle;

    fn connect(
        &self,
        _details: &ConnectionDetails,
        chained: Option<Box<dyn Transport>>,
    ) -> Result<Option<Idle>, ureq::Error> {
        Ok(chained.map(|inner| Idle {
            inner,
            limit: self.0,
        }))
    }
}

#[derive(Debug)]
struct Idle {
    inner: Box<dyn Transport>,
    limit: Duration,
}

impl Transport for Idle {
    fn buffers(&mut self) -> &mut dyn Buffers {
        self.inner.buffers()
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), ureq::Error> {
        self.inner.transmit_output(amount, timeout)
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, ureq::Error> {
        let limit = UreqDuration::Exact(self.limit);
        let timeout = if timeout.after > limit {
            NextTimeout {
                after: limit,
                reason: timeout.reason,
            }
        } else {
            timeout
        };
        self.inner.await_input(timeout)
    }

    fn is_open(&mut self) -> bool {
        self.inner.is_open()
    }

    fn is_tls(&self) -> bool {
        self.inner.is_tls()
    }
}

/// One request, ready to send.
pub(crate) struct Request<'a> {
    pub agent: &'a ureq::Agent,
    pub endpoint: &'a str,
    /// For log lines and a relative `Location`.
    pub origin: &'a str,
    pub accept: &'static str,
    /// The key, for a provider that sends one. `None` sends no
    /// `Authorization` header at all.
    pub key: Option<&'a Secret>,
    pub body: Vec<u8>,
    /// The read timeout, for the sentence that names it.
    pub timeout: Duration,
}

/// The caller's side of a request, as the request's thread sees it: whether
/// the caller is still there.
pub(crate) struct Stop<'a>(pub &'a flume::Receiver<()>);

impl Stop<'_> {
    /// The caller has gone — cancelled, or dropped the call.
    pub(crate) fn now(&self) -> bool {
        matches!(self.0.try_recv(), Err(flume::TryRecvError::Disconnected))
    }

    /// Wait `span`, or less if the caller goes. `true` when it went.
    fn wait(&self, span: Duration) -> bool {
        !matches!(
            self.0.recv_timeout(span),
            Err(flume::RecvTimeoutError::Timeout)
        )
    }
}

/// Send `request`, retrying before the first byte, and read the answer to
/// its end marker. Every piece goes to `forward`; `false` back means the
/// caller has gone.
pub(crate) fn exchange(
    request: &Request,
    format: &mut dyn Format,
    stop: &Stop,
    forward: &mut dyn FnMut(String) -> bool,
) -> Result<Completion, EngineError> {
    checked(request.endpoint)?;
    let mut attempt = 0;
    let response = loop {
        if stop.now() {
            return Err(EngineError::Cancelled);
        }
        let asked = Instant::now();
        let response = match send(request) {
            Ok(response) => response,
            Err(error) => {
                if attempt < RETRIES && worth_retrying(&error) {
                    tracing::debug!(origin = request.origin, attempt, %error, "no connection; trying again");
                    if stop.wait(BACKOFF[attempt]) {
                        return Err(EngineError::Cancelled);
                    }
                    attempt += 1;
                    continue;
                }
                tracing::info!(origin = request.origin, attempt, %error, "the endpoint could not be reached");
                return Err(not_sent(error, request.timeout));
            }
        };
        let status = response.status().as_u16();
        tracing::debug!(
            origin = request.origin,
            status,
            attempt,
            elapsed_ms = asked.elapsed().as_millis(),
            "the endpoint answered"
        );
        match status {
            200..=299 => break response,
            300..=399 => {
                let to_origin = location_origin(&response, request.origin);
                tracing::info!(
                    origin = request.origin,
                    status,
                    ?to_origin,
                    "a redirect, not followed"
                );
                return Err(EngineError::Unavailable(Unavailable::Redirected {
                    status,
                    to_origin,
                }));
            }
            429 | 502 | 503 | 504 => {
                let after = retry_after(&response);
                if attempt < RETRIES {
                    let wait = after
                        .map_or(BACKOFF[attempt], |seconds| {
                            Duration::from_secs(u64::from(seconds))
                        })
                        .min(LONGEST_WAIT);
                    drop(response);
                    if stop.wait(wait) {
                        return Err(EngineError::Cancelled);
                    }
                    attempt += 1;
                    continue;
                }
                if status == 429 {
                    return Err(EngineError::Unavailable(Unavailable::RateLimited {
                        retry_after_s: after,
                    }));
                }
                return Err(EngineError::Unavailable(Unavailable::Refused {
                    status,
                    detail: detail(response),
                }));
            }
            401 | 403 => {
                return Err(EngineError::Unavailable(Unavailable::KeyRejected {
                    status,
                }));
            }
            404 => {
                return Err(EngineError::Unavailable(Unavailable::NotFound {
                    detail: detail(response),
                }));
            }
            _ => {
                return Err(EngineError::Unavailable(Unavailable::Refused {
                    status,
                    detail: detail(response),
                }));
            }
        }
    };
    streamed(response, request, format, stop, forward)
}

/// Read the answer as it arrives.
fn streamed(
    response: ureq::http::Response<ureq::Body>,
    request: &Request,
    format: &mut dyn Format,
    stop: &Stop,
    forward: &mut dyn FnMut(String) -> bool,
) -> Result<Completion, EngineError> {
    let started = Instant::now();
    let mut reader = response.into_body().into_reader();
    let mut lines = Lines::default();
    let mut assembled = Assembled::default();
    let mut first: Option<u128> = None;
    let mut buffer = [0u8; 8192];
    'reading: loop {
        if stop.now() {
            return Err(EngineError::Cancelled);
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|error| read_failed(&error, request.timeout))?;
        if read == 0 {
            break;
        }
        assembled.bytes += read as u64;
        for line in lines.push(&buffer[..read])? {
            if let Some(said) = format.line(&line)? {
                if let Some(piece) = assembled.take(said) {
                    first.get_or_insert_with(|| started.elapsed().as_millis());
                    if !forward(piece) {
                        return Err(EngineError::Cancelled);
                    }
                }
            }
            if assembled.done {
                break 'reading;
            }
        }
    }
    if !assembled.done {
        let rest = lines.rest()?;
        if let Some(said) = format.end(rest.as_deref())? {
            if let Some(piece) = assembled.take(said) {
                if !forward(piece) {
                    return Err(EngineError::Cancelled);
                }
            }
        }
    }
    tracing::debug!(
        origin = request.origin,
        bytes = assembled.bytes,
        pieces = assembled.pieces,
        done = assembled.done,
        first_piece_ms = ?first,
        elapsed_ms = started.elapsed().as_millis(),
        "the answer was read"
    );
    assembled.completion()
}

fn send(request: &Request) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
    let mut builder = request
        .agent
        .post(request.endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", request.accept);
    if let Some(key) = request.key {
        // The one place a key leaves its wrapper (D57).
        builder = builder.header("Authorization", &format!("Bearer {}", key.expose()));
    }
    builder.send(&request.body[..])
}

/// Refuse an endpoint that is not plain `http`/`https` to a bare
/// authority, before anything is opened.
pub(crate) fn checked(endpoint: &str) -> Result<(), EngineError> {
    let Some((scheme, rest)) = endpoint.split_once("://") else {
        return Err(EngineError::Transport(
            "the endpoint is not a URL; only http and https endpoints are asked".to_owned(),
        ));
    };
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")) {
        return Err(EngineError::Transport(format!(
            "{scheme}:// is not HTTP; only http and https endpoints are asked"
        )));
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return Err(EngineError::Transport(
            "the endpoint has no host, or carries credentials in its URL".to_owned(),
        ));
    }
    Ok(())
}

/// A failure before any answer worth a retry: the connection could not be
/// made, the name did not resolve, the TLS handshake failed.
fn worth_retrying(error: &ureq::Error) -> bool {
    use std::io::ErrorKind;
    match error {
        ureq::Error::ConnectionFailed | ureq::Error::HostNotFound | ureq::Error::Tls(_) => true,
        ureq::Error::Timeout(reason) => {
            matches!(reason, ureq::Timeout::Connect | ureq::Timeout::Resolve)
        }
        ureq::Error::Io(error) => matches!(
            error.kind(),
            ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::AddrNotAvailable
        ),
        _ => false,
    }
}

/// A request that got no answer, in ureq's words — except a timeout, which
/// is named in seconds.
fn not_sent(error: ureq::Error, timeout: Duration) -> EngineError {
    match error {
        ureq::Error::Timeout(ureq::Timeout::Connect | ureq::Timeout::Resolve) => {
            EngineError::Transport(format!(
                "could not connect within {} s",
                CONNECT_TIMEOUT.as_secs()
            ))
        }
        ureq::Error::Timeout(_) => silent(timeout),
        other => EngineError::Transport(other.to_string()),
    }
}

/// A read that failed after the answer had begun.
fn read_failed(error: &std::io::Error, timeout: Duration) -> EngineError {
    let timed_out = error.kind() == std::io::ErrorKind::TimedOut
        || error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<ureq::Error>())
            .is_some_and(|inner| matches!(inner, ureq::Error::Timeout(_)));
    if timed_out {
        silent(timeout)
    } else {
        tracing::debug!(%error, "the answer stopped arriving");
        ended_early()
    }
}

fn silent(timeout: Duration) -> EngineError {
    EngineError::Transport(format!("no answer within {} s", timeout.as_secs()))
}

/// `Retry-After` in seconds. An HTTP date is not read: a server that sends
/// one gets the default wait.
fn retry_after(response: &ureq::http::Response<ureq::Body>) -> Option<u32> {
    response
        .headers()
        .get("retry-after")?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// The origin a redirect pointed at — scheme, host and port — and nothing
/// of its path, its query or any credentials in it. A relative `Location`
/// is this endpoint's own origin.
fn location_origin(response: &ureq::http::Response<ureq::Body>, own: &str) -> Option<String> {
    let location = response.headers().get("location")?.to_str().ok()?.trim();
    Some(origin_of(location).unwrap_or_else(|| own.to_owned()))
}

pub(crate) fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    Some(format!("{}://{}", scheme.to_ascii_lowercase(), host))
}

/// At most [`DETAIL_BYTES`] of an error's body, as text. Never the request.
fn detail(response: ureq::http::Response<ureq::Body>) -> String {
    let mut bytes = Vec::new();
    let _ = response
        .into_body()
        .into_reader()
        .take(DETAIL_BYTES)
        .read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_redirect_names_only_an_origin() {
        assert_eq!(
            origin_of("https://user:pw@evil.example:8443/v1/x?key=1#f").as_deref(),
            Some("https://evil.example:8443")
        );
        assert_eq!(origin_of("/relative/path"), None);
    }
}
