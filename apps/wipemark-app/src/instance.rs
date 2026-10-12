//! One Wipemark per data directory: a second launch that only summons
//! hands the summons to the first and exits (D646).
//!
//! The GNOME custom shortcut runs `wipemark --panel` ([`crate::gnome`]).
//! Started while Wipemark runs, that would be a second application — a
//! second window, a second hold on the database, the MCP port stepped
//! over — rather than the panel of the one already there. So the running
//! application listens on a local socket, `<data dir>/wipemark.sock`, and
//! a launch whose whole request is a summons (`--panel`, `--show`, nothing
//! else) connects first: answered, it has done its job and exits 0;
//! unanswered — nothing there, or a socket a killed process left — it
//! starts as it always did, and its own `--panel` opens the panel.
//!
//! Not MCP, which is off by default and is a server for agents; and not a
//! D-Bus name, which would put the summons on the session bus for any
//! process to send. The socket is the user's, mode 0600, in the user's own
//! data directory (or `$XDG_RUNTIME_DIR` when that path is too long for a
//! socket), and it understands two words.
//!
//! The listening is a thread of its own; a summons crosses to the GPUI
//! side over a `flume` channel, as the tray's clicks and the shortcut's
//! presses do.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The socket's file name under the data directory.
pub const FILE: &str = "wipemark.sock";

/// The longest path a Unix socket can be bound at: `sun_path` is 108
/// bytes on Linux, with the terminating NUL.
const SUN_PATH: usize = 107;

/// What a second launch asks the first for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Summons {
    /// `--panel`: the panel, toggled as the shortcut toggles it.
    Panel,
    /// `--show`: the main window, brought forward.
    Show,
}

impl Summons {
    /// The word on the wire. A **format**: an older and a newer build may
    /// talk to each other across an upgrade.
    pub fn word(self) -> &'static str {
        match self {
            Self::Panel => "panel",
            Self::Show => "show",
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        match word.trim() {
            "panel" => Some(Self::Panel),
            "show" => Some(Self::Show),
            _ => None,
        }
    }
}

/// Where the socket for `data_dir` is: in it, or — when that path is too
/// long to bind — in `runtime_dir` under a name made from the data
/// directory, so two data directories are still two instances. `None`
/// when neither fits.
pub fn socket_path(data_dir: &Path, runtime_dir: Option<&Path>) -> Option<PathBuf> {
    let beside = data_dir.join(FILE);
    if beside.as_os_str().len() <= SUN_PATH {
        return Some(beside);
    }
    let runtime = runtime_dir?;
    let named = runtime.join(format!("wipemark-{:016x}.sock", fnv(data_dir)));
    (named.as_os_str().len() <= SUN_PATH).then_some(named)
}

/// FNV-1a over the path's bytes: stable across builds and runs, which a
/// `DefaultHasher` does not promise.
fn fnv(path: &Path) -> u64 {
    use std::os::unix::ffi::OsStrExt as _;
    path.as_os_str()
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        })
}

/// Why a summons was not handed over.
#[derive(Debug)]
pub enum NotForwarded {
    /// Nothing listens there: no instance, or one that was killed.
    NoInstance(std::io::Error),
    /// Something listened and did not say `ok`.
    NoAnswer(String),
}

impl std::fmt::Display for NotForwarded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoInstance(error) => write!(f, "no instance listens: {error}"),
            Self::NoAnswer(said) => write!(f, "the instance did not answer: {said}"),
        }
    }
}

/// Hand `summons` to the instance listening at `path`.
///
/// `Ok` only once it answered `ok` — a summons it took. The wait is
/// bounded: a hung instance must not leave a shortcut's process hanging
/// behind it.
pub fn forward(path: &Path, summons: Summons) -> Result<(), NotForwarded> {
    let mut stream = UnixStream::connect(path).map_err(NotForwarded::NoInstance)?;
    let bound = Some(Duration::from_secs(3));
    let _ = stream.set_read_timeout(bound);
    let _ = stream.set_write_timeout(bound);
    writeln!(stream, "{}", summons.word())
        .map_err(|error| NotForwarded::NoAnswer(error.to_string()))?;
    let mut answer = String::new();
    BufReader::new(&stream)
        .read_line(&mut answer)
        .map_err(|error| NotForwarded::NoAnswer(error.to_string()))?;
    if answer.trim() == "ok" {
        Ok(())
    } else {
        Err(NotForwarded::NoAnswer(answer.trim().to_owned()))
    }
}

/// The listening half, while it lives: dropping it stops nothing (the
/// thread ends with the process), but [`Listening::remove`] takes the file
/// away at quit when it is still ours.
pub struct Listening {
    path: PathBuf,
    /// The file this process bound, by device and inode.
    bound: Option<(u64, u64)>,
}

impl Listening {
    /// Take the socket file away, if it is still the one this process
    /// bound — a later instance may have replaced it.
    pub fn remove(&self) {
        if self.bound.is_some() && identity(&self.path) == self.bound {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// A file's device and inode, read without following a link.
fn identity(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt as _;
    std::fs::symlink_metadata(path)
        .ok()
        .map(|metadata| (metadata.dev(), metadata.ino()))
}

/// Bind at `path` and answer summonses on a thread of their own, each sent
/// on the returned channel. `None` when another instance already answers
/// there, or the socket cannot be made — the application runs either way,
/// it only cannot be summoned this road.
pub fn listen(path: &Path) -> Option<(Listening, flume::Receiver<Summons>)> {
    let listener = match bind(path) {
        Ok(listener) => listener,
        Err(error) => {
            tracing::warn!(%error, "no single-instance socket; a second launch starts a second application");
            return None;
        }
    };
    let (sender, summonses) = flume::unbounded();
    let spawned = std::thread::Builder::new()
        .name("wipemark-instance".to_owned())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                if !answer(stream, &sender) {
                    break;
                }
            }
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "no thread for the single-instance socket");
        return None;
    }
    tracing::info!("listening for a second launch's summons");
    Some((
        Listening {
            path: path.to_path_buf(),
            bound: identity(path),
        },
        summonses,
    ))
}

/// Bind, replacing a socket file nothing answers on — what a killed
/// process leaves behind — and only that: a file that is not a socket, or
/// one an instance answers on, is left where it is.
fn bind(path: &Path) -> std::io::Result<UnixListener> {
    use std::os::unix::fs::{FileTypeExt as _, PermissionsExt as _};

    let listener = match UnixListener::bind(path) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            let stale = std::fs::symlink_metadata(path)
                .map(|metadata| metadata.file_type().is_socket())
                .unwrap_or(false)
                && UnixStream::connect(path).is_err();
            if !stale {
                return Err(error);
            }
            tracing::info!("a socket nothing answers on was left behind; replaced");
            std::fs::remove_file(path)?;
            UnixListener::bind(path)?
        }
        Err(error) => return Err(error),
    };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Read one word, send its summons on, say `ok`. `false` once nothing is
/// listening on the GPUI side any more.
fn answer(stream: UnixStream, sender: &flume::Sender<Summons>) -> bool {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut word = String::new();
    let mut reader = BufReader::new(&stream);
    if reader.read_line(&mut word).is_err() {
        return true;
    }
    let Some(summons) = Summons::from_word(&word) else {
        tracing::debug!("a word on the instance socket that is not a summons");
        let _ = (&stream).write_all(b"unknown\n");
        return true;
    };
    if sender.send(summons).is_err() {
        return false;
    }
    tracing::info!(
        summons = summons.word(),
        "a second launch summoned this one"
    );
    let _ = (&stream).write_all(b"ok\n");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "wm-inst-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");
        root
    }

    /// D646: a summons sent to a running instance arrives on its channel,
    /// and the sender is told it was taken; with nothing listening the
    /// sender is told so, and starts on its own. Red with `forward`
    /// answering Ok without an answer, or the listener not sending on.
    #[test]
    fn a_summons_reaches_the_running_instance() {
        let root = scratch("forward");
        let path = root.join(FILE);
        assert!(matches!(
            forward(&path, Summons::Panel),
            Err(NotForwarded::NoInstance(_))
        ));

        let (listening, summonses) = listen(&path).expect("listening");
        forward(&path, Summons::Panel).expect("the panel handed over");
        forward(&path, Summons::Show).expect("the window handed over");
        let got: Vec<Summons> = (0..2)
            .map(|_| {
                summonses
                    .recv_timeout(Duration::from_secs(5))
                    .expect("a summons")
            })
            .collect();
        assert_eq!(got, [Summons::Panel, Summons::Show]);

        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&path)
            .expect("the socket")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "{mode:o}");

        // A second instance does not take the socket from the first.
        assert!(listen(&path).is_none());
        listening.remove();
        assert!(!path.exists(), "the socket outlived its instance");
        std::fs::remove_dir_all(&root).ok();
    }

    /// A socket a killed process left behind is no instance: the summons
    /// is not handed to it, and the next instance binds in its place. A
    /// file that is not a socket is never removed.
    #[test]
    fn a_stale_socket_is_replaced_and_nothing_else_is() {
        let root = scratch("stale");
        let path = root.join(FILE);
        drop(UnixListener::bind(&path).expect("a socket left behind"));
        assert!(path.exists());
        assert!(matches!(
            forward(&path, Summons::Panel),
            Err(NotForwarded::NoInstance(_))
        ));
        let (_listening, summonses) = listen(&path).expect("bound over the stale one");
        forward(&path, Summons::Panel).expect("handed over");
        assert_eq!(
            summonses.recv_timeout(Duration::from_secs(5)),
            Ok(Summons::Panel)
        );

        let plain = root.join("not-a-socket");
        std::fs::write(&plain, b"mine").expect("write");
        assert!(listen(&plain).is_none());
        assert_eq!(std::fs::read(&plain).expect("still there"), b"mine");
        std::fs::remove_dir_all(&root).ok();
    }

    /// An instance that hears something it does not understand says so,
    /// and the sender does not take that for a yes.
    #[test]
    fn a_word_that_is_not_a_summons_is_not_taken() {
        let root = scratch("word");
        let path = root.join(FILE);
        let (_listening, summonses) = listen(&path).expect("listening");
        let mut stream = UnixStream::connect(&path).expect("connect");
        writeln!(stream, "rm -rf").expect("write");
        let mut answer = String::new();
        BufReader::new(&stream)
            .read_line(&mut answer)
            .expect("read");
        assert_eq!(answer.trim(), "unknown");
        assert!(summonses.try_recv().is_err());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_socket_sits_in_the_data_directory_unless_it_cannot() {
        let data = Path::new("/home/me/.local/share/com.GigLabo.wipemark");
        assert_eq!(
            socket_path(data, Some(Path::new("/run/user/1000"))),
            Some(data.join(FILE))
        );
        let long = PathBuf::from(format!("/home/me/{}", "deep/".repeat(30)));
        let other = PathBuf::from(format!("/home/me/{}x", "deep/".repeat(30)));
        let runtime = Path::new("/run/user/1000");
        let moved = socket_path(&long, Some(runtime)).expect("a path in the runtime dir");
        assert!(moved.starts_with(runtime), "{moved:?}");
        assert_ne!(Some(moved.clone()), socket_path(&other, Some(runtime)));
        assert_eq!(socket_path(&long, Some(runtime)), Some(moved), "stable");
        assert_eq!(socket_path(&long, None), None);
    }

    #[test]
    fn the_words_are_read_back() {
        for summons in [Summons::Panel, Summons::Show] {
            assert_eq!(Summons::from_word(summons.word()), Some(summons));
        }
        assert_eq!(Summons::from_word("settings"), None);
    }
}
