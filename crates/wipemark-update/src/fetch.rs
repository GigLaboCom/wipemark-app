//! The two requests: the manifest, and the artifact a verified offer names.
//!
//! The agent is the model downloader's (`wipemark-models`' `store.rs`):
//! blocking, on a caller-owned thread, the trust roots of the operating
//! system, the proxy settings of the environment. What differs is what an
//! updater owes the person it runs for:
//!
//! * **https only** against the release service, and **no redirect** is
//!   ever followed — a 3xx is a refusal. The JSON comes from its host, and
//!   the artifact from the URL this program built.
//! * **The request carries nothing**: `GET`, the path, `Host`,
//!   `Accept` and `User-Agent: wipemark` — no version, no query, no cookie,
//!   no body (D784). `the_check_sends_nothing_but_the_get` holds it.
//! * **A manifest is read to [`MANIFEST_CAP`]** and no further.
//! * **A download is held to its statement as it streams**: the sha256 is
//!   computed on the way in, it stops one byte past the stated size, and a
//!   file that is not the statement's is deleted before it is ever renamed
//!   into place.

use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sha2::{Digest as _, Sha256};

use crate::manifest::{self, Offer, Served, Shape};
use crate::{Base, MANIFEST_CAP, USER_AGENT};

/// How long a connection may take to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long the head of an answer may take once the request is sent.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// Why a request gave nothing usable, in a machine's terms; the surfaces
/// word it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    /// The host could not be reached: no network, no name, refused.
    #[error("no connection")]
    Connection,
    #[error("timed out")]
    Timeout,
    #[error("the secure connection failed")]
    Tls,
    /// A 3xx: no redirect is followed.
    #[error("redirected, and redirects are not followed")]
    Redirect,
    /// Any other status than 200.
    #[error("answered {0}")]
    Status(u16),
    /// Larger than this program reads.
    #[error("larger than expected")]
    TooLarge,
    /// Not a manifest.
    #[error("{0}")]
    Shape(String),
    /// The body broke off.
    #[error("the answer broke off")]
    Read,
}

impl From<Shape> for FetchError {
    fn from(shape: Shape) -> Self {
        match shape {
            Shape::TooLarge => FetchError::TooLarge,
            Shape::Json(why) => FetchError::Shape(why),
        }
    }
}

impl FetchError {
    fn of(error: &ureq::Error) -> FetchError {
        match error {
            ureq::Error::StatusCode(status) => FetchError::Status(*status),
            ureq::Error::Timeout(_) => FetchError::Timeout,
            ureq::Error::Tls(_) => FetchError::Tls,
            ureq::Error::TooManyRedirects | ureq::Error::RedirectFailed => FetchError::Redirect,
            ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Io(_) => {
                FetchError::Connection
            }
            _ => FetchError::Connection,
        }
    }
}

/// Why a download was not kept. Every one of them leaves nothing behind.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DownloadError {
    #[error("{0}")]
    Fetch(#[from] FetchError),
    /// More bytes came than the statement says: cut at one past, refused.
    #[error("longer than announced")]
    Longer,
    /// Fewer bytes came than the statement says.
    #[error("shorter than announced")]
    Shorter,
    /// The bytes are not the statement's.
    #[error("its sha256 is not the announced one")]
    Sha256,
    /// The file could not be written.
    #[error("it could not be written: {0:?}")]
    Write(io::ErrorKind),
    /// Asked to stop.
    #[error("cancelled")]
    Cancelled,
}

/// The client every request of an update goes through.
pub struct Client {
    agent: ureq::Agent,
    base: Base,
}

impl Client {
    /// A client of `base`: https only against the release service, plain
    /// http only on a loopback base, which only a test or a debug build's
    /// live check names.
    pub fn new(base: Base) -> Client {
        let builder = ureq::Agent::config_builder()
            .https_only(!base.plain())
            .max_redirects(0)
            .http_status_as_error(false)
            .user_agent(USER_AGENT)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT));
        // A proxy from the environment, as the model downloader takes one —
        // but never between this program and its own loopback.
        let builder = if base.plain() {
            builder.proxy(None)
        } else {
            builder
        };
        Client {
            agent: builder.build().into(),
            base,
        }
    }

    pub fn base(&self) -> &Base {
        &self.base
    }

    /// GET the manifest and read it. Blocking.
    pub fn manifest(&self) -> Result<Served, FetchError> {
        let response = self
            .agent
            .get(self.base.manifest_url())
            .header("Accept", "application/json")
            .call()
            .map_err(|error| FetchError::of(&error))?;
        let body = ok_body(response)?;
        if body
            .content_length()
            .is_some_and(|length| length > MANIFEST_CAP as u64)
        {
            return Err(FetchError::TooLarge);
        }
        let mut bytes = Vec::new();
        body.into_reader()
            .take(MANIFEST_CAP as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| FetchError::Read)?;
        Ok(manifest::parse(&bytes)?)
    }

    /// Fetch `offer` into `into`, held to its size and sha256 as it streams.
    ///
    /// The bytes go to `<into>.part` first; only a file that is the
    /// statement's is renamed to `into`, and any other is deleted. A file
    /// already at `into` that is the statement's is kept and not fetched
    /// again. `progress` is told the bytes so far and the total. Blocking.
    pub fn download(
        &self,
        offer: &Offer,
        into: &Path,
        progress: &mut dyn FnMut(u64, u64),
        cancel: &AtomicBool,
    ) -> Result<PathBuf, DownloadError> {
        if already(into, offer) {
            progress(offer.size, offer.size);
            return Ok(into.to_path_buf());
        }
        let part = part_of(into);
        let result = self.fetch_into(offer, &part, progress, cancel);
        match result {
            Ok(()) => {
                fs::rename(&part, into).map_err(|error| {
                    let _ = fs::remove_file(&part);
                    DownloadError::Write(error.kind())
                })?;
                Ok(into.to_path_buf())
            }
            Err(error) => {
                let _ = fs::remove_file(&part);
                Err(error)
            }
        }
    }

    fn fetch_into(
        &self,
        offer: &Offer,
        part: &Path,
        progress: &mut dyn FnMut(u64, u64),
        cancel: &AtomicBool,
    ) -> Result<(), DownloadError> {
        // The URL the offer carries is the one this program built.
        let response = self
            .agent
            .get(&offer.url)
            .call()
            .map_err(|error| FetchError::of(&error))?;
        let body = ok_body(response)?;
        if body
            .content_length()
            .is_some_and(|length| length > offer.size)
        {
            return Err(DownloadError::Longer);
        }
        if let Some(parent) = part.parent() {
            fs::create_dir_all(parent).map_err(|error| DownloadError::Write(error.kind()))?;
        }
        let mut file = File::create(part).map_err(|error| DownloadError::Write(error.kind()))?;
        let mut reader = body.into_reader().take(offer.size + 1);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; 64 * 1024];
        let mut got: u64 = 0;
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(DownloadError::Cancelled);
            }
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(FetchError::Read.into()),
            };
            got += read as u64;
            if got > offer.size {
                return Err(DownloadError::Longer);
            }
            hasher.update(&buffer[..read]);
            file.write_all(&buffer[..read])
                .map_err(|error| DownloadError::Write(error.kind()))?;
            progress(got, offer.size);
        }
        if got < offer.size {
            return Err(DownloadError::Shorter);
        }
        if hasher.finalize().as_slice() != offer.sha256 {
            return Err(DownloadError::Sha256);
        }
        file.sync_all()
            .map_err(|error| DownloadError::Write(error.kind()))?;
        Ok(())
    }
}

/// A 200's body, and every other status refused — a 3xx as a redirect.
fn ok_body(response: ureq::http::Response<ureq::Body>) -> Result<ureq::Body, FetchError> {
    let status = response.status().as_u16();
    match status {
        200 => Ok(response.into_body()),
        300..=399 => Err(FetchError::Redirect),
        other => Err(FetchError::Status(other)),
    }
}

/// `<into>.part`.
fn part_of(into: &Path) -> PathBuf {
    let mut name = into.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    into.with_file_name(name)
}

/// Whether `into` already holds the statement's bytes.
fn already(into: &Path, offer: &Offer) -> bool {
    let Ok(meta) = fs::metadata(into) else {
        return false;
    };
    if !meta.is_file() || meta.len() != offer.size {
        return false;
    }
    let Ok(mut file) = File::open(into) else {
        return false;
    };
    let mut hasher = Sha256::new();
    if io::copy(&mut file, &mut hasher).is_err() {
        return false;
    }
    hasher.finalize().as_slice() == offer.sha256
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::*;
    use crate::keys::Keyring;
    use crate::manifest::Wanted;
    use crate::testing::{Release, Response, Server, Signer};
    use crate::{check, Format, Platform};

    fn wanted() -> Wanted {
        Wanted {
            running: Version::new(0, 1, 0),
            platform: Platform::LinuxX64,
            format: Format::AppImage,
            skipped: None,
        }
    }

    /// A server serving `release` and the manifest that names it.
    fn serving(signer: &Signer, release: &Release) -> Server {
        let server = Server::start(Vec::new());
        server.route(
            "/downloads/wipemark",
            Response::ok(signer.manifest(&server.base(), std::slice::from_ref(release))),
        );
        server.route(&release.path(), Response::ok(release.bytes.clone()));
        server
    }

    fn offer(server: &Server, signer: &Signer) -> Offer {
        check(&Client::new(server.base()), &signer.keyring(), &wanted())
            .expect("a choice")
            .offer
            .expect("an offer")
    }

    /// §4.1's promise, as a test: the request is a bare GET. Red by adding
    /// the version to the user agent, or anything else to the request.
    #[test]
    fn the_check_sends_nothing_but_the_get() {
        let signer = Signer::new();
        let server = serving(&signer, &Release::appimage("0.2.0", b"x"));
        let _ = offer(&server, &signer);
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.target, "/downloads/wipemark");
        assert!(request.body.is_empty());
        let mut headers = request.headers.clone();
        headers.sort();
        assert_eq!(
            headers,
            vec![
                ("accept".to_owned(), "application/json".to_owned()),
                ("host".to_owned(), format!("127.0.0.1:{}", server.port())),
                ("user-agent".to_owned(), "wipemark".to_owned()),
            ]
        );
    }

    #[test]
    fn a_verified_download_lands_whole() {
        let signer = Signer::new();
        let release = Release::appimage("0.2.0", &[7; 300_000]);
        let server = serving(&signer, &release);
        let offer = offer(&server, &signer);
        let dir = tempfile::tempdir().unwrap();
        let into = dir.path().join("Wipemark.AppImage");
        let mut seen = Vec::new();
        let landed = Client::new(server.base())
            .download(
                &offer,
                &into,
                &mut |got, total| seen.push((got, total)),
                &AtomicBool::new(false),
            )
            .expect("downloaded");
        assert_eq!(landed, into);
        assert_eq!(fs::read(&into).unwrap(), release.bytes);
        assert_eq!(seen.last(), Some(&(300_000, 300_000)));
        assert!(!part_of(&into).exists());
        // Asked again: what is there is the statement's, and nothing is
        // fetched.
        let before = server.connections();
        Client::new(server.base())
            .download(&offer, &into, &mut |_, _| {}, &AtomicBool::new(false))
            .expect("kept");
        assert_eq!(server.connections(), before);
    }

    /// The bytes are the statement's, or nothing is kept. Red by skipping
    /// the comparison.
    #[test]
    fn a_download_whose_sha256_differs_is_refused_and_removed() {
        let signer = Signer::new();
        let release = Release::appimage("0.2.0", b"the real bytes");
        let server = serving(&signer, &release);
        let offer = offer(&server, &signer);
        // One byte flipped, the length kept.
        server.route(&release.path(), Response::ok(b"the real bytez".to_vec()));
        let dir = tempfile::tempdir().unwrap();
        let into = dir.path().join("Wipemark.AppImage");
        assert_eq!(
            Client::new(server.base()).download(
                &offer,
                &into,
                &mut |_, _| {},
                &AtomicBool::new(false)
            ),
            Err(DownloadError::Sha256)
        );
        assert!(!into.exists());
        assert!(!part_of(&into).exists());
    }

    /// The size cap. Red by reading to the end.
    #[test]
    fn a_download_longer_than_announced_is_cut_and_refused() {
        let signer = Signer::new();
        let release = Release::appimage("0.2.0", b"announced");
        let server = serving(&signer, &release);
        let offer = offer(&server, &signer);
        let dir = tempfile::tempdir().unwrap();
        let into = dir.path().join("Wipemark.AppImage");
        // The announced bytes and then more: the hash of the first part
        // matches, so only the cap can refuse it.
        let mut longer = release.bytes.clone();
        longer.extend_from_slice(&[0; 4096]);
        // Announced by its length, and refused before a byte; then sent
        // chunked, its length unknown until read — the cap is what cuts it.
        for response in [Response::ok(longer.clone()), Response::chunked(longer)] {
            server.route(&release.path(), response);
            assert_eq!(
                Client::new(server.base()).download(
                    &offer,
                    &into,
                    &mut |_, _| {},
                    &AtomicBool::new(false)
                ),
                Err(DownloadError::Longer)
            );
            assert!(!into.exists());
            assert!(!part_of(&into).exists());
        }
        // And shorter.
        server.route(&release.path(), Response::ok(b"announ".to_vec()));
        assert_eq!(
            Client::new(server.base()).download(
                &offer,
                &into,
                &mut |_, _| {},
                &AtomicBool::new(false)
            ),
            Err(DownloadError::Shorter)
        );
    }

    /// Privacy: a manifest cannot steer this computer to another host.
    /// Server B is where the manifest says the file is; it is never
    /// reached. Red by trusting the manifest's `url`.
    #[test]
    fn an_asset_on_another_host_is_refused_before_any_request() {
        let signer = Signer::new();
        let release = Release::appimage("0.2.0", b"bytes");
        let elsewhere = Server::start(Vec::new());
        elsewhere.route(&release.path(), Response::ok(release.bytes.clone()));
        let server = Server::start(Vec::new());
        let mut manifest: serde_json::Value =
            serde_json::from_str(&signer.manifest(&server.base(), std::slice::from_ref(&release)))
                .unwrap();
        manifest["versions"][0]["assets"][0]["url"] = elsewhere
            .base()
            .artifact_url("0.2.0", "linux-x64", &release.file)
            .into();
        server.route("/downloads/wipemark", Response::ok(manifest.to_string()));
        server.route(&release.path(), Response::ok(release.bytes.clone()));

        let client = Client::new(server.base());
        let choice = check(&client, &signer.keyring(), &wanted()).expect("a choice");
        if let Some(offer) = &choice.offer {
            let dir = tempfile::tempdir().unwrap();
            let _ = client.download(
                offer,
                &dir.path().join("x"),
                &mut |_, _| {},
                &AtomicBool::new(false),
            );
        }
        assert_eq!(elsewhere.connections(), 0, "the other host was reached");
        assert_eq!(choice.offer, None);
    }

    /// The JSON comes only from its host. Red by allowing redirects.
    #[test]
    fn the_manifest_is_never_followed_through_a_redirect() {
        let signer = Signer::new();
        let release = Release::appimage("0.2.0", b"x");
        let target = serving(&signer, &release);
        let server = Server::start(Vec::new());
        server.route(
            "/downloads/wipemark",
            Response::redirect(&target.base().manifest_url()),
        );
        assert_eq!(
            check(&Client::new(server.base()), &signer.keyring(), &wanted()),
            Err(crate::Refusal::Fetch(FetchError::Redirect))
        );
        assert_eq!(target.connections(), 0);
    }

    /// Memory. Red by reading without the cap.
    #[test]
    fn a_manifest_over_the_cap_is_refused() {
        let server = Server::start(Vec::new());
        // A valid manifest padded past the cap with white space JSON allows,
        // announced by its length and then sent chunked.
        let mut body = br#"{"versions":[]}"#.to_vec();
        body.extend(std::iter::repeat_n(b' ', MANIFEST_CAP));
        for response in [Response::ok(body.clone()), Response::chunked(body)] {
            server.route("/downloads/wipemark", response);
            assert_eq!(
                Client::new(server.base()).manifest().err(),
                Some(FetchError::TooLarge)
            );
        }
        // At the cap it is read.
        let mut body = br#"{"versions":[]}"#.to_vec();
        body.extend(std::iter::repeat_n(b' ', MANIFEST_CAP - body.len()));
        server.route("/downloads/wipemark", Response::ok(body));
        assert!(Client::new(server.base()).manifest().is_ok());
    }

    #[test]
    fn a_status_or_no_connection_is_said_as_such() {
        let server = Server::start(vec![(
            "/downloads/wipemark".to_owned(),
            Response::status(503),
        )]);
        assert_eq!(
            Client::new(server.base()).manifest().err(),
            Some(FetchError::Status(503))
        );
        // A port nothing listens on.
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = closed.local_addr().unwrap().port();
        drop(closed);
        assert_eq!(
            Client::new(Base::loopback(port)).manifest().err(),
            Some(FetchError::Connection)
        );
        // Not a manifest.
        let server = Server::start(vec![(
            "/downloads/wipemark".to_owned(),
            Response::ok("<html>"),
        )]);
        assert!(matches!(
            Client::new(server.base()).manifest(),
            Err(FetchError::Shape(_))
        ));
        let _ = Keyring::default();
    }

    #[test]
    fn a_cancelled_download_leaves_nothing() {
        let signer = Signer::new();
        let release = Release::appimage("0.2.0", &[1; 200_000]);
        let server = serving(&signer, &release);
        let offer = offer(&server, &signer);
        let dir = tempfile::tempdir().unwrap();
        let into = dir.path().join("x");
        assert_eq!(
            Client::new(server.base()).download(
                &offer,
                &into,
                &mut |_, _| {},
                &AtomicBool::new(true)
            ),
            Err(DownloadError::Cancelled)
        );
        assert!(!into.exists());
        assert!(!part_of(&into).exists());
    }
}
