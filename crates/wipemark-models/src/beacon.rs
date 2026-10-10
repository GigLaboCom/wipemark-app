//! How a process on this machine finds the running application (D52).
//!
//! The application's MCP server binds a port — the one asked for, or up to
//! ten above it when that one is taken — and that port is a fact only the
//! running process knows. `wipemark-cli rewrite` has to reach it to use
//! the application's loaded model rather than load a second copy. So the
//! server leaves a **beacon** under the data directory while it listens:
//! `mcp.json`, three facts — the process id, the port, and the loopback
//! address to dial.
//!
//! # Loopback, or no beacon at all
//!
//! The beacon is for another process **on this machine**, and only ever
//! names a loopback address: a server bound to loopback names it, a server
//! bound to a wildcard (`0.0.0.0`, `::`) is reachable on loopback and names
//! that, and a server bound to one specific address off this machine
//! writes none ([`Beacon::for_server`]). A reader refuses a beacon that
//! names anything else ([`Beacon::reachable`]) — a document handed to "the
//! application" must not travel to an address somebody wrote into a file.
//!
//! # Stale is the ordinary case
//!
//! A crash leaves the file behind; so does a `kill -9`. A beacon is a hint
//! that something *was* listening, and [`Beacon::alive`] is the first of
//! the reader's checks — the process must still exist — before it dials
//! and asks the server who it is. The file is written and removed only by
//! the process it names ([`Beacon::remove_if_ours`]), so a second instance
//! stopping its server cannot take the first one's beacon away.
//!
//! Under the data directory, so `WIPEMARK_DATA_DIR` moves it with
//! everything else, and so another OS user's application — which has a
//! data directory of its own — is never found.

use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The file's name under the data directory. A format: the application
/// writes it and the CLI reads it, and an older CLI must still find a
/// newer application's.
pub const FILE: &str = "mcp.json";

/// What the running application's MCP server leaves for other processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beacon {
    /// The process that is listening.
    pub pid: u32,
    /// The port it actually bound — not necessarily the one it was asked
    /// for.
    pub port: u16,
    /// Where to dial it: always a loopback address when this crate wrote
    /// it.
    pub address: IpAddr,
}

impl Beacon {
    /// The beacon for a server bound to `bind` on `port` in this process,
    /// or `None` when it is not reachable from this machine's loopback.
    pub fn for_server(bind: IpAddr, port: u16) -> Option<Beacon> {
        let address = match bind {
            IpAddr::V4(v4) if v4.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(v6) if v6.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
            address if address.is_loopback() => address,
            _ => return None,
        };
        Some(Beacon {
            pid: std::process::id(),
            port,
            address,
        })
    }

    /// Where to dial, when the beacon names this machine — `None` for
    /// anything else, whoever wrote it.
    pub fn reachable(&self) -> Option<SocketAddr> {
        self.address
            .is_loopback()
            .then(|| SocketAddr::new(self.address, self.port))
    }

    /// Whether the process it names is running. A process id can be
    /// reused, so this is the first check and not the last: the reader
    /// still asks the server who it is.
    pub fn alive(&self) -> bool {
        use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

        let pid = Pid::from_u32(self.pid);
        let mut system = System::new();
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            ProcessRefreshKind::new(),
        );
        system.process(pid).is_some()
    }

    /// Read the beacon at `path`. `None` when there is none, or when the
    /// file is not one — either way there is nothing to dial.
    pub fn read(path: &Path) -> Option<Beacon> {
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Write it at `path`, whole or not at all: to a file beside it first,
    /// then renamed over, so a reader never sees half a beacon.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let staged = path.with_extension(format!("json.{}.part", self.pid));
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        std::fs::write(&staged, bytes)?;
        std::fs::rename(&staged, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&staged);
        })
    }

    /// Remove the beacon at `path` if it names `pid` — the process that
    /// wrote it — and leave anybody else's alone. `true` when one was
    /// removed.
    pub fn remove_if_ours(path: &Path, pid: u32) -> bool {
        match Beacon::read(path) {
            Some(beacon) if beacon.pid == pid => std::fs::remove_file(path).is_ok(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "wipemark-beacon-{label}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch");
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Loopback names itself, a wildcard names loopback, and an address
    /// off this machine names nothing — for the writer and the reader
    /// alike.
    #[test]
    fn a_beacon_names_loopback_or_nothing() {
        let v4: IpAddr = "127.0.0.1".parse().expect("an address");
        let v6: IpAddr = "::1".parse().expect("an address");
        for (bind, said) in [
            ("127.0.0.1", Some(v4)),
            ("127.0.0.7", Some("127.0.0.7".parse().expect("an address"))),
            ("::1", Some(v6)),
            ("0.0.0.0", Some(v4)),
            ("::", Some(v6)),
            ("192.168.1.20", None),
            ("10.0.0.1", None),
            ("fe80::1", None),
        ] {
            let bind: IpAddr = bind.parse().expect("an address");
            let beacon = Beacon::for_server(bind, 5056);
            assert_eq!(beacon.map(|beacon| beacon.address), said, "{bind}");
            if let Some(beacon) = beacon {
                assert_eq!(beacon.pid, std::process::id());
                assert_eq!(
                    beacon.reachable(),
                    Some(SocketAddr::new(beacon.address, 5056))
                );
            }
        }
        // Whoever wrote it: a beacon naming another machine is not dialled.
        let elsewhere = Beacon {
            pid: 1,
            port: 5056,
            address: "192.168.1.20".parse().expect("an address"),
        };
        assert_eq!(elsewhere.reachable(), None);
    }

    /// Written whole, read back the same, and removed only by the process
    /// it names.
    #[test]
    fn a_beacon_is_removed_only_by_its_writer() {
        let scratch = Scratch::new("ours");
        let path = scratch.0.join("data").join(FILE);
        let beacon = Beacon::for_server("127.0.0.1".parse().expect("an address"), 5057)
            .expect("loopback has a beacon");
        beacon.write(&path).expect("written");
        assert_eq!(Beacon::read(&path), Some(beacon));
        assert_eq!(
            std::fs::read_dir(path.parent().expect("a folder"))
                .expect("listed")
                .count(),
            1,
            "the staged copy was left behind"
        );

        assert!(!Beacon::remove_if_ours(&path, beacon.pid.wrapping_add(1)));
        assert_eq!(Beacon::read(&path), Some(beacon), "another's was removed");
        assert!(Beacon::remove_if_ours(&path, beacon.pid));
        assert_eq!(Beacon::read(&path), None);
        assert!(
            !Beacon::remove_if_ours(&path, beacon.pid),
            "nothing to remove"
        );

        std::fs::write(&path, b"{not a beacon").expect("written");
        assert_eq!(Beacon::read(&path), None, "a broken file is no beacon");
    }

    /// This process is alive; a process id nobody has is not.
    #[test]
    fn a_beacon_knows_whether_its_process_runs() {
        let ours = Beacon::for_server("127.0.0.1".parse().expect("an address"), 5056)
            .expect("loopback has a beacon");
        assert!(ours.alive());
        let gone = Beacon {
            pid: u32::MAX - 7,
            ..ours
        };
        assert!(!gone.alive(), "pid {} is somebody's", gone.pid);
    }
}
