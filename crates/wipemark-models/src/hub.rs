//! What a Hugging Face link names, asked of the Hub before anything is
//! downloaded (D835): the commit a revision stands at, whether the
//! repository is gated, its licence, every file with its size and — for a
//! file kept in LFS — its sha256, and the header of one GGUF, read from its
//! first megabytes with ranged requests.
//!
//! Read-only: nothing here downloads a model, writes a file or records a
//! row. A catalogue entry drafted from it ([`entry_draft`]) is text for a
//! person to finish and commit; the catalogue itself stays compiled in.
//!
//! The Hub's own answers are metadata, and the bytes decide what a thing
//! is (the rules): the verdicts come from the header as
//! [`Header::read_from`] reads it, never from the `gguf` summary the API
//! offers. The agent is the downloader's — a connect timeout, the product's
//! user agent, and no `Authorization` header — so a gated repository is
//! said to be gated, never asked for with a credential.

use std::io::Cursor;
use std::time::Duration;

use serde::Deserialize;

use crate::gguf::{GgufError, Header, MAX_METADATA};

/// Where the Hub answers.
pub const ENDPOINT: &str = "https://huggingface.co";

/// Names another Hub — a mirror, or a test's server on loopback. `https://`
/// anywhere, `http://` on this machine only, as a catalogue URL.
pub const ENDPOINT_VAR: &str = "WIPEMARK_HF_ENDPOINT";

/// How much of a file is asked for first. A header with a vocabulary of a
/// quarter of a million tokens is about 7 MiB; most fit.
pub const FIRST_READ: u64 = 8 << 20;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// The longest API answer read: a tree of thousands of files is a few
/// hundred kilobytes.
const LONGEST_ANSWER: u64 = 16 << 20;

/// Why the Hub could not be asked, or did not say.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HubError {
    /// The endpoint is neither `https://` nor `http://` on loopback.
    #[error("not an https:// address, nor http:// on this machine: {0}")]
    Endpoint(String),
    /// The repository, the revision or the file is not there — or is
    /// private, which the Hub answers the same way to an anonymous caller.
    #[error("not found: {0}")]
    NotFound(String),
    /// The Hub wants an account for this: a gated file.
    #[error("refused without an account: {0}")]
    Refused(String),
    /// Any other status.
    #[error("{url} answered {status}")]
    Status { url: String, status: u16 },
    /// No answer: the network, a timeout, TLS.
    #[error("{url}: {cause}")]
    Network { url: String, cause: String },
    /// An answer this reader does not understand.
    #[error("{url}: {reason}")]
    Answer { url: String, reason: String },
    /// A ranged read came back whole, or short: the server ignored the
    /// range, and a model's every byte is not read to see its header.
    #[error("{0} did not answer the range it was asked for")]
    NoRange(String),
    /// The header, as far as it was read.
    #[error("{0}")]
    Header(GgufError),
}

/// A link to a model on the Hub, as a person pastes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// `owner/name`.
    pub repo: String,
    /// A branch, a tag or a commit; `None` is the default branch.
    pub revision: Option<String>,
    /// A file in the repository, when the link names one.
    pub path: Option<String>,
}

impl Link {
    /// Read `text`: `https://huggingface.co/<owner>/<name>` (with or
    /// without the scheme, `hf.co` too), the same with `/tree/<rev>`,
    /// `/blob/<rev>/<file>` or `/resolve/<rev>/<file>`, the catalogue's
    /// `hf://<owner>/<name>@<rev>/<file>`, or a bare `<owner>/<name>`. A
    /// query and a fragment are dropped. `None` for anything else — a
    /// dataset, a space, another host.
    #[must_use]
    pub fn parse(text: &str) -> Option<Link> {
        let text = text.trim();
        let text = text.split(['?', '#']).next().unwrap_or_default();
        if let Some(rest) = text.strip_prefix("hf://") {
            let (owner, rest) = rest.split_once('/')?;
            let (name, path) = rest.split_once('/').unwrap_or((rest, ""));
            let repo = format!("{owner}/{name}");
            let (repo, revision) = match repo.split_once('@') {
                Some((repo, revision)) => (repo.to_owned(), Some(revision.to_owned())),
                None => (repo, None),
            };
            return Link::checked(repo, revision, non_empty(path));
        }
        let without_scheme = text
            .strip_prefix("https://")
            .or_else(|| text.strip_prefix("http://"))
            .unwrap_or(text);
        let segments: Vec<&str> = without_scheme
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        let segments = match segments.first() {
            Some(host) if host.contains('.') || host.contains(':') => {
                let host = host.to_ascii_lowercase();
                if !matches!(
                    host.as_str(),
                    "huggingface.co" | "www.huggingface.co" | "hf.co"
                ) {
                    return None;
                }
                &segments[1..]
            }
            _ => &segments[..],
        };
        match segments {
            [owner, name] => Link::checked(format!("{owner}/{name}"), None, None),
            [owner, name, "tree", revision, ..] => Link::checked(
                format!("{owner}/{name}"),
                Some((*revision).to_owned()),
                None,
            ),
            [owner, name, "blob" | "resolve", revision, path @ ..] if !path.is_empty() => {
                Link::checked(
                    format!("{owner}/{name}"),
                    Some((*revision).to_owned()),
                    Some(path.join("/")),
                )
            }
            _ => None,
        }
    }

    fn checked(repo: String, revision: Option<String>, path: Option<String>) -> Option<Link> {
        let (owner, name) = repo.split_once('/')?;
        let plain = |part: &str| {
            !part.is_empty()
                && !part.starts_with('.')
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        if !plain(owner) || !plain(name) || matches!(owner, "datasets" | "spaces" | "api") {
            return None;
        }
        if revision.as_deref().is_some_and(|revision| !plain(revision)) {
            return None;
        }
        if path
            .as_deref()
            .is_some_and(|path| path.split('/').any(|part| part.is_empty() || part == ".."))
        {
            return None;
        }
        Some(Link {
            repo,
            revision,
            path,
        })
    }
}

fn non_empty(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_owned())
}

/// One file of a repository at a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoFile {
    pub path: String,
    pub size: u64,
    /// The file's sha256 — the LFS object id. `None` for a file kept in git
    /// itself, whose id is a git blob's and not its bytes' hash.
    pub sha256: Option<String>,
}

impl RepoFile {
    /// The file's name, its last path segment.
    #[must_use]
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// A GGUF file, by its name.
    #[must_use]
    pub fn is_gguf(&self) -> bool {
        self.path.to_ascii_lowercase().ends_with(".gguf")
    }

    /// `Some((part, of))` for a split GGUF, `name-00001-of-00003.gguf`.
    #[must_use]
    pub fn split_part(&self) -> Option<(u32, u32)> {
        let stem = self.name().strip_suffix(".gguf")?;
        let (rest, of) = stem.rsplit_once("-of-")?;
        let (_, part) = rest.rsplit_once('-')?;
        if part.len() != 5 || of.len() != 5 {
            return None;
        }
        Some((part.parse().ok()?, of.parse().ok()?))
    }
}

/// A repository at one commit, as the Hub describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub repo: String,
    /// The 40-character commit the asked revision stands at.
    pub commit: String,
    /// `Some("auto" | "manual")` when an account must accept the
    /// repository's licence before a file is served.
    pub gated: Option<String>,
    /// The model card's licence: its `license_name` when `license` is
    /// `other`.
    pub license: Option<String>,
    /// Every file, in the Hub's order.
    pub files: Vec<RepoFile>,
}

impl Snapshot {
    /// The GGUF files.
    #[must_use]
    pub fn ggufs(&self) -> Vec<&RepoFile> {
        self.files.iter().filter(|file| file.is_gguf()).collect()
    }

    /// The file at `path`.
    #[must_use]
    pub fn file(&self, path: &str) -> Option<&RepoFile> {
        self.files.iter().find(|file| file.path == path)
    }

    /// `file` and the other parts of its split, in order — or `file` alone.
    #[must_use]
    pub fn parts_of<'a>(&'a self, file: &'a RepoFile) -> Vec<&'a RepoFile> {
        let Some((_, of)) = file.split_part() else {
            return vec![file];
        };
        let prefix = |file: &RepoFile| {
            file.path
                .rsplit_once("-of-")
                .and_then(|(rest, _)| rest.rsplit_once('-'))
                .map(|(prefix, _)| prefix.to_owned())
        };
        let mine = prefix(file);
        let mut parts: Vec<&RepoFile> = self
            .files
            .iter()
            .filter(|other| other.split_part().is_some_and(|(_, n)| n == of))
            .filter(|other| prefix(other) == mine)
            .collect();
        parts.sort_by_key(|other| other.split_part().map(|(part, _)| part));
        parts
    }

    /// The catalogue's URL of `file` at this commit.
    #[must_use]
    pub fn url_of(&self, file: &RepoFile) -> String {
        format!("hf://{}@{}/{}", self.repo, self.commit, file.path)
    }
}

/// What `/api/models/<repo>/revision/<rev>` says, as far as it is read.
#[derive(Debug, Deserialize)]
struct Revision {
    sha: String,
    #[serde(default)]
    gated: serde_json::Value,
    #[serde(default, rename = "cardData")]
    card: Option<Card>,
}

#[derive(Debug, Deserialize)]
struct Card {
    license: Option<serde_json::Value>,
    license_name: Option<String>,
}

/// One entry of `/api/models/<repo>/tree/<commit>`.
#[derive(Debug, Deserialize)]
struct TreeEntry {
    #[serde(rename = "type")]
    kind: String,
    path: String,
    #[serde(default)]
    size: u64,
    lfs: Option<Lfs>,
}

#[derive(Debug, Deserialize)]
struct Lfs {
    oid: String,
    size: u64,
}

/// The commit, the gate and the licence out of a revision's answer.
fn read_revision(
    body: &str,
    url: &str,
) -> Result<(String, Option<String>, Option<String>), HubError> {
    let revision: Revision = serde_json::from_str(body).map_err(|error| HubError::Answer {
        url: url.to_owned(),
        reason: error.to_string(),
    })?;
    if revision.sha.len() != 40 || !revision.sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(HubError::Answer {
            url: url.to_owned(),
            reason: format!("not a commit: {}", revision.sha),
        });
    }
    let gated = match revision.gated {
        serde_json::Value::String(gate) => Some(gate),
        serde_json::Value::Bool(true) => Some("auto".to_owned()),
        _ => None,
    };
    let license = revision.card.and_then(|card| {
        let named = card.license_name.filter(|name| !name.trim().is_empty());
        match card.license {
            Some(serde_json::Value::String(license)) if license != "other" => Some(license),
            Some(serde_json::Value::Array(licenses)) => licenses
                .iter()
                .filter_map(serde_json::Value::as_str)
                .find(|license| *license != "other")
                .map(str::to_owned)
                .or(named),
            _ => named,
        }
    });
    Ok((revision.sha, gated, license))
}

/// The files out of a tree's answer.
fn read_tree(body: &str, url: &str) -> Result<Vec<RepoFile>, HubError> {
    let entries: Vec<TreeEntry> = serde_json::from_str(body).map_err(|error| HubError::Answer {
        url: url.to_owned(),
        reason: error.to_string(),
    })?;
    Ok(entries
        .into_iter()
        .filter(|entry| entry.kind == "file")
        .map(|entry| {
            let sha256 = entry
                .lfs
                .as_ref()
                .map(|lfs| lfs.oid.to_ascii_lowercase())
                .filter(|oid| oid.len() == 64 && oid.chars().all(|c| c.is_ascii_hexdigit()));
            RepoFile {
                size: entry.lfs.as_ref().map_or(entry.size, |lfs| lfs.size),
                path: entry.path,
                sha256,
            }
        })
        .collect())
}

/// Read a header from the front of a file of `size` bytes, asking `fetch`
/// for the bytes from `from` up to (not including) `to`: [`FIRST_READ`]
/// first, then twice as much while the header is cut short, never past the
/// file or [`MAX_METADATA`]. Returns the header and how much was read.
pub fn read_header(
    size: u64,
    mut fetch: impl FnMut(u64, u64) -> Result<Vec<u8>, HubError>,
) -> Result<(Header, u64), HubError> {
    let ceiling = size.min(MAX_METADATA);
    let mut bytes: Vec<u8> = Vec::new();
    let mut want = FIRST_READ.min(ceiling);
    loop {
        let have = bytes.len() as u64;
        if want > have {
            bytes.extend(fetch(have, want)?);
        }
        let read = bytes.len() as u64;
        match Header::read_from(Cursor::new(&bytes), read) {
            Ok(header) => return Ok((header, read)),
            Err(GgufError::Truncated) if read < ceiling => want = (read * 2).min(ceiling),
            Err(error) => return Err(HubError::Header(error)),
        }
    }
}

/// Asks the Hub.
pub struct Hub {
    endpoint: String,
    agent: ureq::Agent,
}

impl Hub {
    /// A Hub at `endpoint`: `https://`, or `http://` on loopback.
    pub fn new(endpoint: &str) -> Result<Hub, HubError> {
        let endpoint = endpoint.trim().trim_end_matches('/');
        let loopback = ["http://127.0.0.1", "http://localhost", "http://[::1]"]
            .iter()
            .any(|host| {
                endpoint
                    .strip_prefix(host)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with([':', '/']))
            });
        if !(endpoint.starts_with("https://") || loopback) || endpoint.contains('@') {
            return Err(HubError::Endpoint(endpoint.to_owned()));
        }
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .user_agent(concat!("wipemark/", env!("CARGO_PKG_VERSION")))
            .build();
        Ok(Hub {
            endpoint: endpoint.to_owned(),
            agent: config.into(),
        })
    }

    /// The Hub [`ENDPOINT_VAR`] names, or [`ENDPOINT`].
    pub fn from_env() -> Result<Hub, HubError> {
        match std::env::var(ENDPOINT_VAR) {
            Ok(endpoint) if !endpoint.trim().is_empty() => Hub::new(&endpoint),
            _ => Hub::new(ENDPOINT),
        }
    }

    /// The repository `link` names, at the commit its revision stands at.
    ///
    /// The API answers a repository that is not there as it answers a
    /// private one — 401 to a caller with no account — so both are
    /// [`HubError::NotFound`] here; a gated repository's metadata is public.
    pub fn snapshot(&self, link: &Link) -> Result<Snapshot, HubError> {
        let unknown = |error: HubError| match error {
            HubError::Refused(url) => HubError::NotFound(url),
            other => other,
        };
        let revision = link.revision.as_deref().unwrap_or("main");
        let url = format!(
            "{}/api/models/{}/revision/{revision}",
            self.endpoint, link.repo
        );
        let (commit, gated, license) = read_revision(&self.text(&url).map_err(unknown)?, &url)?;
        let url = format!(
            "{}/api/models/{}/tree/{commit}?recursive=true",
            self.endpoint, link.repo
        );
        let files = read_tree(&self.text(&url).map_err(unknown)?, &url)?;
        Ok(Snapshot {
            repo: link.repo.clone(),
            commit,
            gated,
            license,
            files,
        })
    }

    /// The header of `file`, read from its front ([`read_header`]).
    pub fn header(&self, snapshot: &Snapshot, file: &RepoFile) -> Result<(Header, u64), HubError> {
        let url = format!(
            "{}/{}/resolve/{}/{}",
            self.endpoint, snapshot.repo, snapshot.commit, file.path
        );
        read_header(file.size, |from, to| self.range(&url, from, to))
    }

    fn text(&self, url: &str) -> Result<String, HubError> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| failed(url, &error))?;
        response
            .body_mut()
            .with_config()
            .limit(LONGEST_ANSWER)
            .read_to_string()
            .map_err(|error| failed(url, &error))
    }

    fn range(&self, url: &str, from: u64, to: u64) -> Result<Vec<u8>, HubError> {
        let mut response = self
            .agent
            .get(url)
            .header("Range", &format!("bytes={from}-{}", to - 1))
            .call()
            .map_err(|error| failed(url, &error))?;
        if response.status().as_u16() != 206 {
            return Err(HubError::NoRange(url.to_owned()));
        }
        let bytes = response
            .body_mut()
            .with_config()
            .limit(to - from + 1)
            .read_to_vec()
            .map_err(|error| failed(url, &error))?;
        if bytes.len() as u64 != to - from {
            return Err(HubError::NoRange(url.to_owned()));
        }
        Ok(bytes)
    }
}

fn failed(url: &str, error: &ureq::Error) -> HubError {
    match error {
        ureq::Error::StatusCode(404) => HubError::NotFound(url.to_owned()),
        ureq::Error::StatusCode(401 | 403) => HubError::Refused(url.to_owned()),
        ureq::Error::StatusCode(status) => HubError::Status {
            url: url.to_owned(),
            status: *status,
        },
        other => HubError::Network {
            url: url.to_owned(),
            cause: other.to_string(),
        },
    }
}

/// A catalogue entry for `parts` (a file, or every part of a split one),
/// drafted from the Hub's metadata and the header of the first: the URLs
/// pinned to the snapshot's commit, each sha256 and size as the Hub states
/// them, the quantization and the default context read off the header, the
/// memory by the catalogue's recipe (`mem_mb`, the caller's estimate),
/// `"status": "experimental"`. Its tier, its languages and its notes are a
/// person's to write: a draft carries tier 1, `["multi"]` and no notes.
#[must_use]
pub fn entry_draft(
    snapshot: &Snapshot,
    parts: &[&RepoFile],
    header: &Header,
    ctx: u32,
    mem_mb: u64,
) -> serde_json::Value {
    let first = parts.first().map_or("", |file| file.name());
    let stem = first.strip_suffix(".gguf").unwrap_or(first);
    let stem = match parts.first().and_then(|file| file.split_part()) {
        Some(_) => stem
            .rsplit_once("-of-")
            .and_then(|(rest, _)| rest.rsplit_once('-'))
            .map_or(stem, |(prefix, _)| prefix),
        None => stem,
    };
    let mut id = String::new();
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() || c == '.' {
            id.push(c.to_ascii_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id = id.trim_matches('-').trim_start_matches('.').to_owned();
    let quant = header.quant(first);
    let name = header
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| stem.to_owned());
    let display = match &quant {
        Some(quant) => format!("{name} ({quant})"),
        None => name,
    };
    let files: Vec<serde_json::Value> = parts
        .iter()
        .map(|file| {
            serde_json::json!({
                "url": snapshot.url_of(file),
                "sha256": file.sha256,
                "size_bytes": file.size,
            })
        })
        .collect();
    serde_json::json!({
        "id": id,
        "display": display,
        "roles": ["rewrite"],
        "format": "gguf",
        "status": "experimental",
        "files": files,
        "mem": { "min_ram_mb": mem_mb, "min_vram_mb": mem_mb },
        "ctx_default": ctx,
        "quant": quant,
        "quality_tier": 1,
        "license": snapshot.license.clone().unwrap_or_else(|| "unknown".to_owned()),
        "langs": ["multi"],
        "vendor": "open-llm",
        "notes": "",
    })
}

#[cfg(test)]
mod tests {
    use super::{
        entry_draft, read_header, read_revision, read_tree, Hub, HubError, Link, RepoFile,
        Snapshot, FIRST_READ,
    };
    use crate::gguf::{synthetic, GgufError, Meta};
    use crate::manifest::Manifest;

    const COMMIT: &str = "b738967d6da3441a7d1c71fcec15e49e32e16ecd";
    const SHA: &str = "84ff34f5834b929043f5ae74799e8d562d4056998f6a5923ceb44831e8ad84d6";

    fn link(repo: &str, revision: Option<&str>, path: Option<&str>) -> Link {
        Link {
            repo: repo.to_owned(),
            revision: revision.map(str::to_owned),
            path: path.map(str::to_owned),
        }
    }

    /// Every way a person pastes a model, and what is not one.
    #[test]
    fn a_link_is_read_however_it_was_pasted() {
        let repo = "unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF";
        for (text, read) in [
            (
                "https://huggingface.co/unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF",
                Some(link(repo, None, None)),
            ),
            (
                "huggingface.co/unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF/",
                Some(link(repo, None, None)),
            ),
            (
                "unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF",
                Some(link(repo, None, None)),
            ),
            (
                "https://hf.co/unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF/tree/main",
                Some(link(repo, Some("main"), None)),
            ),
            (
                "https://huggingface.co/unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF/blob/main/NVIDIA-Nemotron-3.5-Bolt-E4B-UD-Q4_K_XL.gguf?download=true",
                Some(link(
                    repo,
                    Some("main"),
                    Some("NVIDIA-Nemotron-3.5-Bolt-E4B-UD-Q4_K_XL.gguf"),
                )),
            ),
            (
                "https://huggingface.co/unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF/resolve/b738967d6da3441a7d1c71fcec15e49e32e16ecd/q/x.gguf",
                Some(link(repo, Some(COMMIT), Some("q/x.gguf"))),
            ),
            (
                "hf://unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF@b738967d6da3441a7d1c71fcec15e49e32e16ecd/x.gguf",
                Some(link(repo, Some(COMMIT), Some("x.gguf"))),
            ),
            (
                "hf://unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF",
                Some(link(repo, None, None)),
            ),
            ("https://github.com/unsloth/x", None),
            ("https://huggingface.co/datasets/owner/name", None),
            ("https://huggingface.co/spaces/owner/name", None),
            ("https://huggingface.co/unsloth", None),
            ("unsloth", None),
            ("hf://owner/../etc/x.gguf", None),
            ("https://huggingface.co/o/n/blob/main/../x.gguf", None),
            ("", None),
        ] {
            assert_eq!(Link::parse(text), read, "{text}");
        }
    }

    /// The Hub's revision answer: the commit, the gate, the licence the card
    /// names — its `license_name` when `license` is `other`.
    #[test]
    fn a_revision_says_its_commit_its_gate_and_its_licence() {
        let read = |body: &str| read_revision(body, "u");
        assert_eq!(
            read(&format!(
                r#"{{"sha":"{COMMIT}","gated":false,"cardData":{{"license":"other","license_name":"openmdw-1.1"}}}}"#
            )),
            Ok((COMMIT.to_owned(), None, Some("openmdw-1.1".to_owned())))
        );
        assert_eq!(
            read(&format!(
                r#"{{"sha":"{COMMIT}","gated":"manual","cardData":{{"license":"gemma"}}}}"#
            )),
            Ok((
                COMMIT.to_owned(),
                Some("manual".to_owned()),
                Some("gemma".to_owned())
            ))
        );
        assert_eq!(
            read(&format!(r#"{{"sha":"{COMMIT}"}}"#)),
            Ok((COMMIT.to_owned(), None, None))
        );
        assert!(matches!(
            read(r#"{"sha":"main"}"#),
            Err(HubError::Answer { .. })
        ));
        assert!(matches!(read("<html>"), Err(HubError::Answer { .. })));
    }

    /// A tree's files: the LFS object id is the sha256 and its size the
    /// file's; a file in git itself has no sha256 here; folders are not
    /// files.
    #[test]
    fn a_tree_lists_files_with_the_sha256_lfs_states() {
        let body = format!(
            r#"[{{"type":"file","path":"README.md","size":58954,"oid":"aaaa"}},
               {{"type":"directory","path":"q","size":0}},
               {{"type":"file","path":"q/x.gguf","size":135,"lfs":{{"oid":"{SHA}","size":5060482240,"pointerSize":135}}}}]"#
        );
        assert_eq!(
            read_tree(&body, "u"),
            Ok(vec![
                RepoFile {
                    path: "README.md".to_owned(),
                    size: 58954,
                    sha256: None
                },
                RepoFile {
                    path: "q/x.gguf".to_owned(),
                    size: 5_060_482_240,
                    sha256: Some(SHA.to_owned())
                },
            ])
        );
    }

    /// A header is read from as little of the file as holds it: the first
    /// read, then more while it is cut short, from where the last stopped —
    /// and a file whose header never ends is refused at the file's end.
    #[test]
    fn a_header_is_read_from_the_front_of_the_file_only() {
        // A header longer than the first read, as a large vocabulary makes
        // one: here a description the reader steps over.
        let filler = "x".repeat((FIRST_READ + (3 << 20)) as usize);
        let file = synthetic(
            399,
            &[
                ("general.architecture", Meta::Text("gemma4")),
                ("general.description", Meta::Text(&filler)),
                (
                    "tokenizer.chat_template",
                    Meta::Text("{{ '<|turn>' }}{{ '<turn|>' }}"),
                ),
            ],
        );
        let size = file.len() as u64 + (5 << 30);
        let mut asked = Vec::new();
        let (header, read) = read_header(size, |from, to| {
            asked.push((from, to));
            let to = (to as usize).min(file.len());
            Ok(file[from as usize..to].to_vec())
        })
        .expect("a header");
        assert_eq!(header.architecture.as_deref(), Some("gemma4"));
        assert_eq!(asked, vec![(0, FIRST_READ), (FIRST_READ, 2 * FIRST_READ)]);
        assert_eq!(read, file.len() as u64);

        let cut = &file[..1000];
        assert_eq!(
            read_header(cut.len() as u64, |from, to| Ok(cut
                [from as usize..to as usize]
                .to_vec())),
            Err(HubError::Header(GgufError::Truncated))
        );
    }

    /// An endpoint is https, or http on this machine — never a credential.
    #[test]
    fn the_endpoint_is_https_or_loopback() {
        for good in [
            "https://huggingface.co",
            "https://mirror.example/",
            "http://127.0.0.1:8080",
            "http://localhost:9",
        ] {
            assert!(Hub::new(good).is_ok(), "{good}");
        }
        for bad in [
            "http://huggingface.co",
            "http://localhost.evil.example",
            "https://user:token@huggingface.co",
            "ftp://x",
        ] {
            assert!(matches!(Hub::new(bad), Err(HubError::Endpoint(_))), "{bad}");
        }
    }

    fn snapshot(files: &[(&str, u64)]) -> Snapshot {
        Snapshot {
            repo: "unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF".to_owned(),
            commit: COMMIT.to_owned(),
            gated: None,
            license: Some("openmdw-1.1".to_owned()),
            files: files
                .iter()
                .map(|(path, size)| RepoFile {
                    path: (*path).to_owned(),
                    size: *size,
                    sha256: Some(SHA.to_owned()),
                })
                .collect(),
        }
    }

    /// A split model is drafted with every part in order, under one id.
    #[test]
    fn a_split_file_brings_its_parts() {
        let snapshot = snapshot(&[
            ("M-Q4_K_M-00002-of-00002.gguf", 2),
            ("M-Q4_K_M-00001-of-00002.gguf", 1),
            ("M-Q8_0-00001-of-00002.gguf", 3),
            ("M-BF16.gguf", 4),
        ]);
        let first = snapshot
            .file("M-Q4_K_M-00001-of-00002.gguf")
            .expect("a part");
        let parts: Vec<&str> = snapshot
            .parts_of(first)
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(
            parts,
            [
                "M-Q4_K_M-00001-of-00002.gguf",
                "M-Q4_K_M-00002-of-00002.gguf"
            ]
        );
        let whole = snapshot.file("M-BF16.gguf").expect("a file");
        assert_eq!(snapshot.parts_of(whole).len(), 1);
        assert_eq!(snapshot.ggufs().len(), 4);
    }

    /// The draft is an entry the catalogue's own parser takes — pinned to
    /// the commit, the Hub's sha256 and size, experimental — and its id is
    /// the file's name, made plain.
    #[test]
    fn a_draft_is_an_entry_the_catalogue_takes() {
        let snapshot = snapshot(&[(
            "NVIDIA-Nemotron-3.5-Bolt-E4B-UD-Q4_K_XL.gguf",
            5_060_482_240,
        )]);
        let file = &snapshot.files[0];
        let header = crate::gguf::Header {
            name: Some("NVIDIA-Nemotron-3.5-Bolt-E4B".to_owned()),
            ..crate::gguf::Header::default()
        };
        let draft = entry_draft(&snapshot, &[file], &header, 8192, 6144);
        assert_eq!(draft["id"], "nvidia-nemotron-3.5-bolt-e4b-ud-q4-k-xl");
        assert_eq!(
            draft["display"],
            "NVIDIA-Nemotron-3.5-Bolt-E4B (UD-Q4_K_XL)"
        );
        assert_eq!(
            draft["files"][0]["url"],
            format!("hf://unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF@{COMMIT}/NVIDIA-Nemotron-3.5-Bolt-E4B-UD-Q4_K_XL.gguf")
        );
        assert_eq!(draft["status"], "experimental");
        let manifest = Manifest::parse(&format!(r#"{{"schema": 1, "models": [{draft}]}}"#))
            .expect("the catalogue takes the draft");
        let entry = &manifest.models[0];
        assert_eq!(entry.files[0].sha256.as_deref(), Some(SHA));
        assert_eq!(entry.files[0].size_bytes, 5_060_482_240);
        assert_eq!(entry.license, "openmdw-1.1");
    }
}
