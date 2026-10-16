//! One Wipemark per data directory: one process holds it, and a second
//! launch hands its summons to that one or ends (D646, D790–D794).
//!
//! **The hold.** Before the database, the batch queue, the journal, the
//! MCP server or a window, the application takes an exclusive lock on
//! `<data dir>/wipemark.lock` ([`hold`], D791). Two processes on one data
//! directory would both read the queue's rows and both run its next job,
//! both write the journal, both serve MCP (F33). The lock is the kernel's
//! (`flock`), so it goes with the process however the process ends —
//! `kill -9` and a crash included — and the file is never removed. The
//! command line never takes it: it finds the application by its beacon
//! and writes only its own journal row.
//!
//! **The summons.** The GNOME custom shortcut runs `wipemark --panel`
//! ([`crate::gnome`]); on macOS a terminal, a script or a launcher runs the
//! bundle's own executable, which LaunchServices does not keep to one
//! process (D790). So the running application listens on a local socket,
//! `<data dir>/wipemark.sock`, and a launch whose whole request is a
//! summons (`--panel`, `--show`, or nothing at all) connects first:
//! answered, it has done its job and exits 0; unanswered — nothing there,
//! or a socket a killed process left — it takes the hold and starts, and
//! its own `--panel` opens the panel. A launch that finds the hold taken
//! and asked for more than a summons brings the main window forward and
//! ends, saying why ([`admit`], D792).
//!
//! Not MCP, which is off by default and is a server for agents; and not a
//! D-Bus name, which would put the summons on the session bus for any
//! process to send. The socket is the user's, mode 0600, in the user's own
//! data directory (or the user's own runtime directory when that path is
//! too long for a socket — `$XDG_RUNTIME_DIR` on Linux, `$TMPDIR` on
//! macOS), and it understands two words.
//!
//! The listening is a thread of its own, started right after the hold is
//! taken (D793); a summons crosses to the GPUI side over a `flume`
//! channel, as the tray's clicks and the shortcut's presses do.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// The socket's file name under the data directory.
pub const FILE: &str = "wipemark.sock";

/// The lock's file name under the data directory (D791).
pub const LOCK: &str = "wipemark.lock";

/// The longest path a Unix socket can be bound at, without the
/// terminating NUL: `sun_path` is 108 bytes on Linux and 104 on macOS.
#[cfg(target_os = "macos")]
const SUN_PATH: usize = 103;
#[cfg(not(target_os = "macos"))]
const SUN_PATH: usize = 107;

/// How long a launch that finds the hold taken goes on knocking before it
/// says the holder did not answer: long enough for an instance that is
/// starting to bind its socket, or one that is quitting to let go (D793).
pub const PATIENCE: Duration = Duration::from_secs(5);

/// The pause between two knocks.
const KNOCK: Duration = Duration::from_millis(100);

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

/// The user's own directory for a socket that does not fit beside the
/// data directory: `$XDG_RUNTIME_DIR` on Linux; on macOS `$TMPDIR`, the
/// per-user `/var/folders/…/T/` launchd hands every process (D790). Never
/// a shared `/tmp`: without the variable there is no such directory.
pub fn runtime_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    const VARIABLE: &str = "TMPDIR";
    #[cfg(not(target_os = "macos"))]
    const VARIABLE: &str = "XDG_RUNTIME_DIR";
    std::env::var_os(VARIABLE)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
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

/// This process's hold on its data directory: the open lock file (D791).
#[derive(Debug)]
pub struct Held {
    file: File,
}

impl Held {
    /// Keep the hold until the process ends. The kernel lets go then,
    /// however it ends; nothing before that does — not a quit handler, not
    /// GPUI's `run` returning while a job's thread still writes.
    pub fn for_the_life_of_the_process(self) {
        static HELD: OnceLock<File> = OnceLock::new();
        if HELD.set(self.file).is_err() {
            tracing::warn!("the data directory was held twice by one process");
        }
    }
}

/// What asking for the data directory came to.
#[derive(Debug)]
pub enum Hold {
    /// This process holds it, and alone may open the queue, the journal
    /// and the MCP server over it.
    Held(Held),
    /// Another process holds it — the pid it wrote, when that could be
    /// read.
    Taken { pid: Option<u32> },
    /// No lock could be taken at all: a directory that cannot be written,
    /// a filesystem without locks. The application starts, as it starts
    /// without a socket, and says so in the log (D794).
    Unavailable(std::io::Error),
}

/// Ask for `data_dir`: an exclusive, non-blocking lock on its
/// [`LOCK`] file, made if missing, mode 0600 (D791). Held, the file says
/// this process's pid, for the log of a launch that finds it taken. The
/// file is never removed — a lock file removed while another process opens
/// it is two holders.
pub fn hold(data_dir: &Path) -> Hold {
    use std::os::unix::fs::OpenOptionsExt as _;

    if let Err(error) = std::fs::create_dir_all(data_dir) {
        return Hold::Unavailable(error);
    }
    let path = data_dir.join(LOCK);
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) => return Hold::Unavailable(error),
    };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.set_len(0);
            let _ = writeln!(&file, "{}", std::process::id());
            Hold::Held(Held { file })
        }
        Err(TryLockError::WouldBlock) => Hold::Taken {
            pid: std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| text.trim().parse().ok()),
        },
        Err(TryLockError::Error(error)) => Hold::Unavailable(error),
    }
}

/// How a launch goes on once it has knocked and asked for its data
/// directory (D792).
#[derive(Debug)]
pub enum Admission {
    /// Everything it asked for was handed to the instance that holds the
    /// data directory. It exits 0.
    HandedOver(Summons),
    /// Another instance holds the data directory and did not take
    /// everything this launch asked for. It says why and exits 2.
    Refused(Refusal),
    /// This process starts: it holds the data directory, or there is no
    /// hold to be had.
    Start(Start),
}

/// Why a launch that found the data directory held ends without starting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// It asked for a file, a page, a profile or the walk-through, which
    /// only a launch that starts the application takes. The holder's main
    /// window was brought forward.
    MoreThanASummons,
    /// The holder did not answer within [`PATIENCE`], and did not let go.
    NoAnswer,
}

/// What a starting process keeps: its hold, and the socket it answers
/// summonses on.
pub struct Start {
    pub held: Option<Held>,
    pub listening: Option<(Listening, flume::Receiver<Summons>)>,
}

impl std::fmt::Debug for Start {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Start")
            .field("held", &self.held.is_some())
            .field("listening", &self.listening.is_some())
            .finish()
    }
}

/// Knock, then ask for the data directory (D646, D791–D793).
///
/// `summons` is what this launch would hand a running instance — `None`
/// for a launch that asks for more than a summons. A summons is handed
/// over first, before anything opens the database. Otherwise the hold is
/// asked for: held, this process binds its socket at once and starts;
/// taken, it knocks again — the summons, or `show` for a launch that asked
/// for more — until the holder answers or lets go, for at most `patience`.
pub fn admit(
    summons: Option<Summons>,
    data_dir: Option<&Path>,
    socket: Option<&Path>,
    patience: Duration,
) -> Admission {
    if let (Some(summons), Some(path)) = (summons, socket) {
        match forward(path, summons) {
            Ok(()) => return Admission::HandedOver(summons),
            Err(error) => tracing::info!(%error, "no running instance took the summons"),
        }
    }
    let Some(data_dir) = data_dir else {
        return Admission::Start(start(None, socket));
    };
    let deadline = Instant::now() + patience;
    let knocking = summons.unwrap_or(Summons::Show);
    let mut told = false;
    loop {
        match hold(data_dir) {
            Hold::Held(held) => return Admission::Start(start(Some(held), socket)),
            Hold::Unavailable(error) => {
                tracing::warn!(%error, "the data directory could not be locked; a second launch is not kept out");
                return Admission::Start(start(None, socket));
            }
            Hold::Taken { pid } if !told => {
                tracing::info!(?pid, "another instance holds this data directory");
                told = true;
            }
            Hold::Taken { .. } => {}
        }
        if let Some(path) = socket {
            if forward(path, knocking).is_ok() {
                return match summons {
                    Some(summons) => Admission::HandedOver(summons),
                    None => Admission::Refused(Refusal::MoreThanASummons),
                };
            }
        }
        if Instant::now() >= deadline {
            return Admission::Refused(Refusal::NoAnswer);
        }
        std::thread::sleep(KNOCK);
    }
}

/// The process starts: it binds its socket now, before the database, so a
/// summons that arrives while it starts waits on the channel (D793).
fn start(held: Option<Held>, socket: Option<&Path>) -> Start {
    Start {
        held,
        listening: socket.and_then(listen),
    }
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

    /// D790: a data directory whose socket path fits Linux's 107 bytes but
    /// not macOS's 103 — a long home directory's Application Support is
    /// 103 — is still answered: on macOS the socket goes to the runtime
    /// directory. Red with Linux's limit on macOS: the bind fails, and a
    /// second launch starts a second application.
    #[test]
    fn a_socket_that_does_not_fit_beside_goes_to_the_runtime_directory() {
        let root = scratch("long");
        // Short enough for `wipemark-<hash>.sock`; the hash is this test's
        // own data directory's, so nothing else's socket is there.
        let runtime = std::env::temp_dir();
        // `<data>/wipemark.sock` exactly 104 bytes long.
        let base = root.join("d");
        let pad = 104 - base.as_os_str().len() - 1 - FILE.len();
        let data = PathBuf::from(format!("{}{}", base.display(), "x".repeat(pad)));
        assert_eq!(data.join(FILE).as_os_str().len(), 104);
        std::fs::create_dir_all(&data).expect("mkdir");

        let path = socket_path(&data, Some(&runtime)).expect("a socket path");
        let (listening, summonses) = listen(&path).expect("bound");
        forward(&path, Summons::Show).expect("handed over");
        assert_eq!(
            summonses.recv_timeout(Duration::from_secs(5)),
            Ok(Summons::Show)
        );
        listening.remove();
        std::fs::remove_dir_all(&root).ok();
    }

    /// D791: one process holds a data directory; a second ask is told it
    /// is taken, and by whom; once the first lets go the next one holds
    /// it. Red with the lock not taken: two holders, two queues.
    #[test]
    fn one_process_holds_a_data_directory() {
        let root = scratch("hold");
        let Hold::Held(first) = hold(&root) else {
            panic!("the first ask holds it");
        };
        match hold(&root) {
            Hold::Taken { pid } => assert_eq!(pid, Some(std::process::id())),
            other => panic!("a second holder: {other:?}"),
        }
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(root.join(LOCK))
            .expect("the lock file")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "{mode:o}");

        drop(first);
        assert!(
            matches!(hold(&root), Hold::Held(_)),
            "let go with its holder"
        );
        assert!(root.join(LOCK).exists(), "the file is never removed");
        std::fs::remove_dir_all(&root).ok();
    }

    /// F33, D790–D793: whatever a second launch on one data directory asks
    /// for, it never starts beside the first. A summons — `--panel`,
    /// `--show`, or a plain launch — is handed over and the launch exits 0;
    /// a launch that asks for more brings the window forward and is
    /// refused; a holder that does not answer is not started beside. Red
    /// on macOS with the summons left to Linux, and anywhere with the
    /// lock not taken: each of those launches would start.
    #[test]
    fn a_second_launch_hands_over_or_ends_and_never_starts() {
        let root = scratch("admit");
        let data = root.join("data");
        let socket = data.join(FILE);
        let patience = Duration::from_millis(300);
        let a = |summons| admit(summons, Some(&data), Some(&socket), patience);

        let Admission::Start(first) = a(Some(Summons::Show)) else {
            panic!("the first launch starts");
        };
        assert!(first.held.is_some(), "the first holds the data directory");
        let (listening, summonses) = first.listening.expect("and answers on its socket");
        let summoned = || summonses.recv_timeout(Duration::from_secs(5));

        assert!(matches!(
            a(Some(Summons::Panel)),
            Admission::HandedOver(Summons::Panel)
        ));
        assert_eq!(summoned(), Ok(Summons::Panel));
        assert!(
            matches!(a(Some(Summons::Show)), Admission::HandedOver(Summons::Show)),
            "a plain second launch brings the window forward"
        );
        assert_eq!(summoned(), Ok(Summons::Show));
        assert!(matches!(
            a(None),
            Admission::Refused(Refusal::MoreThanASummons)
        ));
        assert_eq!(summoned(), Ok(Summons::Show), "the window brought forward");

        // A holder that cannot be reached: refused, not started beside it.
        listening.remove();
        assert!(matches!(
            a(Some(Summons::Panel)),
            Admission::Refused(Refusal::NoAnswer)
        ));
        assert!(matches!(a(None), Admission::Refused(Refusal::NoAnswer)));

        // Once the holder has gone, the next launch starts.
        drop(first.held);
        let Admission::Start(next) = a(Some(Summons::Panel)) else {
            panic!("the next launch starts");
        };
        assert!(next.held.is_some() && next.listening.is_some());
        std::fs::remove_dir_all(&root).ok();
    }

    /// A launch that finds the holder quitting waits for it to let go and
    /// starts, rather than telling the person to try again (D793).
    #[test]
    fn a_launch_waits_for_a_holder_that_is_letting_go() {
        let root = scratch("letgo");
        let Hold::Held(first) = hold(&root) else {
            panic!("held");
        };
        let letting_go = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(first);
        });
        let admitted = admit(
            Some(Summons::Panel),
            Some(&root),
            Some(&root.join(FILE)),
            Duration::from_secs(5),
        );
        letting_go.join().expect("joined");
        assert!(
            matches!(&admitted, Admission::Start(start) if start.held.is_some()),
            "{admitted:?}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_words_are_read_back() {
        for summons in [Summons::Panel, Summons::Show] {
            assert_eq!(Summons::from_word(summons.word()), Some(summons));
        }
        assert_eq!(Summons::from_word("settings"), None);
    }
}
