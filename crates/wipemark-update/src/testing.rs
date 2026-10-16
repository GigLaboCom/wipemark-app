//! What a test of an update needs and production never has: a key pair made
//! at run time, a manifest signed with it, and a server on the loopback
//! address that records every request and counts every connection.
//!
//! Compiled for this crate's tests, and for another crate's tests through
//! the `test-support` feature — which only a `[dev-dependencies]` entry
//! turns on, so no build that ships carries it. No private key is ever
//! committed: [`Signer::new`] makes one with `ring` each time.

use std::collections::HashMap;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair as _};
use sha2::{Digest as _, Sha256};

use crate::keys::{KeyId, Keyring, PublicKey};
use crate::statement::{Signature, Statement};
use crate::{Base, Format, Platform, CHANNEL};

/// An Ed25519 key pair made for one test.
pub struct Signer {
    pair: Ed25519KeyPair,
    public: PublicKey,
}

impl Default for Signer {
    fn default() -> Self {
        Self::new()
    }
}

impl Signer {
    pub fn new() -> Signer {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).expect("a key pair");
        let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("its own pkcs8");
        let raw: [u8; 32] = pair.public_key().as_ref().try_into().expect("32 bytes");
        Signer {
            pair,
            public: PublicKey::new(raw),
        }
    }

    pub fn id(&self) -> KeyId {
        self.public.id()
    }

    pub fn public(&self) -> PublicKey {
        self.public.clone()
    }

    /// A keyring holding this key alone.
    pub fn keyring(&self) -> Keyring {
        Keyring::of([self.public()])
    }

    /// `text`'s signature, as the manifest carries it.
    pub fn sign_text(&self, text: &str) -> String {
        let signed = self.pair.sign(text.as_bytes());
        let bytes: [u8; 64] = signed.as_ref().try_into().expect("64 bytes");
        Signature::new(self.id(), bytes).to_text()
    }

    /// One asset of the manifest, signed, at the URL `base` builds.
    pub fn asset(&self, base: &Base, release: &Release) -> serde_json::Value {
        let sha256 = hex::encode(Sha256::digest(&release.bytes));
        let platform = release.platform.id();
        let text = Statement {
            channel: CHANNEL,
            version: &release.version,
            platform,
            format: release.format.token(),
            file: &release.file,
            size: release.bytes.len() as u64,
            sha256: &sha256,
        }
        .text()
        .expect("a statement");
        serde_json::json!({
            "os": release.platform.os(),
            "arch": release.platform.arch(),
            "format": release.format.token(),
            "filename": release.file,
            "url": base.artifact_url(&release.version, platform, &release.file),
            "size": release.bytes.len(),
            "sha256": sha256,
            "sig": self.sign_text(&text),
            "built_at": "2026-10-16T11:01:02.056Z",
        })
    }

    /// The release service's manifest of `releases`, every asset signed:
    /// one version entry per version, in the order first named.
    pub fn manifest(&self, base: &Base, releases: &[Release]) -> String {
        let mut versions: Vec<serde_json::Value> = Vec::new();
        for release in releases {
            let asset = self.asset(base, release);
            match versions
                .iter_mut()
                .find(|entry| entry["version"] == release.version.as_str())
            {
                Some(entry) => entry["assets"].as_array_mut().expect("assets").push(asset),
                None => versions.push(serde_json::json!({
                    "version": release.version,
                    "released_at": "2026-10-16T11:01:02.056Z",
                    "channel": CHANNEL,
                    "status": "complete",
                    "expected": ["macos-aarch64", "linux-x64"],
                    "assets": [asset],
                })),
            }
        }
        serde_json::json!({
            "slug": "wipemark",
            "updated_at": "2026-10-16T11:01:02.056Z",
            "latest_by_platform": {},
            "versions": versions,
        })
        .to_string()
    }
}

/// One artifact of a test release: what it is called and what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub platform: Platform,
    pub format: Format,
    pub file: String,
    pub bytes: Vec<u8>,
}

impl Release {
    pub fn dmg(version: &str, bytes: &[u8]) -> Release {
        Release {
            version: version.to_owned(),
            platform: Platform::MacosAarch64,
            format: Format::Dmg,
            file: format!("Wipemark-{version}-arm64.dmg"),
            bytes: bytes.to_vec(),
        }
    }

    pub fn appimage(version: &str, bytes: &[u8]) -> Release {
        Release {
            version: version.to_owned(),
            platform: Platform::LinuxX64,
            format: Format::AppImage,
            file: format!("Wipemark-{version}-x86_64.AppImage"),
            bytes: bytes.to_vec(),
        }
    }

    pub fn deb(version: &str, bytes: &[u8]) -> Release {
        Release {
            version: version.to_owned(),
            platform: Platform::LinuxX64,
            format: Format::Deb,
            file: format!("wipemark_{version}_amd64.deb"),
            bytes: bytes.to_vec(),
        }
    }

    /// Where the release service serves it, relative to the base.
    pub fn path(&self) -> String {
        format!(
            "/wipemark/{}/{}/{}",
            self.version,
            self.platform.id(),
            self.file
        )
    }
}

/// A canned answer.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Sent chunked, with no `Content-Length` — a body whose length the
    /// client learns only by reading it.
    pub chunked: bool,
}

impl Response {
    pub fn ok(body: impl Into<Vec<u8>>) -> Response {
        Response {
            status: 200,
            headers: Vec::new(),
            body: body.into(),
            chunked: false,
        }
    }

    /// A 200 sent chunked: no length is announced before the bytes.
    pub fn chunked(body: impl Into<Vec<u8>>) -> Response {
        Response {
            chunked: true,
            ..Response::ok(body)
        }
    }

    pub fn status(status: u16) -> Response {
        Response {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            chunked: false,
        }
    }

    pub fn redirect(location: &str) -> Response {
        Response {
            status: 302,
            headers: vec![("Location".to_owned(), location.to_owned())],
            body: Vec::new(),
            chunked: false,
        }
    }
}

/// One request as it arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    /// The request target: path and query, as sent.
    pub target: String,
    /// Every header, its name lowercased, in the order sent.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// A server on `127.0.0.1` that answers from a table, one connection at a
/// time, and keeps every request.
pub struct Server {
    port: u16,
    connections: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<Request>>>,
    routes: Arc<Mutex<HashMap<String, Response>>>,
}

impl Server {
    /// Listen, answering `routes` by path (a 404 for any other).
    pub fn start(routes: Vec<(String, Response)>) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("an address").port();
        let server = Server {
            port,
            connections: Arc::default(),
            requests: Arc::default(),
            routes: Arc::new(Mutex::new(routes.into_iter().collect())),
        };
        let (connections, requests, routes) = (
            server.connections.clone(),
            server.requests.clone(),
            server.routes.clone(),
        );
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                connections.fetch_add(1, Ordering::SeqCst);
                let _ = answer(stream, &requests, &routes);
            }
        });
        server
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// This server as an update base.
    pub fn base(&self) -> Base {
        Base::loopback(self.port)
    }

    /// Answer `path` with `response` from now on.
    pub fn route(&self, path: &str, response: Response) {
        self.routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path.to_owned(), response);
    }

    /// How many connections were accepted.
    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// Every request so far.
    pub fn requests(&self) -> Vec<Request> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

fn answer(
    stream: TcpStream,
    requests: &Mutex<Vec<Request>>,
    routes: &Mutex<HashMap<String, Response>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or_default().to_owned();
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.trim_end().split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
        }
    }
    let length: usize = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    let path = target.split('?').next().unwrap_or_default().to_owned();
    requests
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(Request {
            method,
            target,
            headers,
            body,
        });
    let response = routes
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&path)
        .cloned()
        .unwrap_or_else(|| Response::status(404));
    let mut stream = stream;
    let mut head = format!("HTTP/1.1 {} X\r\nConnection: close\r\n", response.status);
    if response.chunked {
        head.push_str("Transfer-Encoding: chunked\r\n");
    } else {
        head.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    }
    for (name, value) in &response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if response.chunked {
        for chunk in response.body.chunks(16 * 1024) {
            stream.write_all(format!("{:x}\r\n", chunk.len()).as_bytes())?;
            stream.write_all(chunk)?;
            stream.write_all(b"\r\n")?;
        }
        stream.write_all(b"0\r\n\r\n")?;
    } else {
        stream.write_all(&response.body)?;
    }
    stream.flush()
}

/// A staged copy that names nothing on disk, for a surface's own tests of
/// what it offers once one is ready.
pub fn prepared() -> crate::install::Prepared {
    crate::install::Prepared::for_tests()
}
