//! What this machine can hold, and which catalogue entry that makes the
//! default.
//!
//! Two halves, deliberately separated: [`Host::probe`] reads the
//! machine, and every function below is **pure** and takes a [`Host`] by
//! value. A policy that probed as it decided could only be tested on the
//! machine running the test, which is the same as not being tested — the
//! rules here are about 12 GB cards and 16 GB laptops, and CI is neither.
//!
//! # Honesty
//!
//! `None` in this module always means *unknown*, never zero. There is no
//! portable way to ask a machine how much video memory it has without
//! linking a vendor driver, so [`Host::vram_mb`] is filled in where it
//! can be — a unified-memory Mac, where the answer is the system RAM,
//! and an NVIDIA machine with the driver's own tool on `PATH` — and is
//! left unknown everywhere else. A discrete AMD or Intel GPU therefore
//! reads as unknown rather than as absent. That is a **deliberate gap**,
//! recorded here rather than papered over with a guess: the wrong
//! answer would be a model refused on a machine that could run it.
//!
//! [`fit`] answers on RAM alone, because RAM is the number that decides
//! whether a model can run at all — a machine with enough of it can
//! always fall back to the CPU. Video memory decides how *fast*, which
//! is not a question this module claims to answer.

use std::time::Duration;

use crate::manifest::{Manifest, ModelEntry, Role};

/// Total-VRAM ceiling (MiB) at or below which a discrete GPU counts as
/// constrained.
///
/// 13000 and not 12288, because cards do not report their marketing
/// size: a 12 GB RTX 4070 reports about 12282 MiB and a 12 GB RTX 3060
/// reports 12288, while the next tier up reports about 16376. Any value
/// between roughly 12.3 and 16.3 GiB separates the two classes; this one
/// sits clear of both edges. Taken, with the reasoning, from
/// `mnemoria-lvkb`'s `NVIDIA_CONSTRAINED_VRAM_MAX_MB`.
pub const CONSTRAINED_VRAM_MAX_MB: u64 = 13_000;

/// Total-RAM ceiling (MiB) at or below which a unified-memory machine —
/// Apple silicon — counts as constrained.
///
/// 17000 and not 16384, for the same reason: a 16 GB Mac reports exactly
/// 16384 MiB and the next configuration up is 18432.
pub const CONSTRAINED_UNIFIED_RAM_MAX_MB: u64 = 17_000;

/// Share of total RAM a model may claim before the fit is called tight.
/// Three quarters leaves the operating system, this application and the
/// document being worked on somewhere to live.
const COMFORTABLE_SHARE: u64 = 4;
const COMFORTABLE_OF: u64 = 3;

/// Share of the pool a *default* may claim on a constrained machine:
/// half.
///
/// `mnemoria-lvkb`'s rule, generalised over a catalogue that is data
/// rather than two named ids: a 16 GB Mac and a 12 GB card are each
/// handed the smaller build because the larger one leaves nothing
/// beside it, never because the smaller one is better. Half, and not
/// the three quarters above, because on these machines the pool is the
/// whole of what the model, its context and every other process share
/// — and the two figures land on either side of the real hardware:
/// 9216 MB is under three quarters of 16384 and over half of it.
const CONSTRAINED_SHARE: u64 = 2;

/// How long the NVIDIA probe is given before it is treated as absent. A
/// driver in a bad state can leave `nvidia-smi` hanging, and a Settings
/// window that will not open is a worse failure than an unknown number.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// What this machine has.
///
/// A plain value, probed once and then passed by copy, so that every
/// decision below is testable against a machine the test describes
/// rather than the machine the test happens to run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Host {
    /// Total physical RAM, MiB. Zero only when the probe failed.
    pub total_ram_mb: u64,
    /// RAM free right now, MiB. A moment, not a capacity — reported so a
    /// surface can say "close some things first", never used to refuse.
    pub available_ram_mb: u64,
    /// Device-local video memory, MiB. `None` is **unknown**: no probe on
    /// this platform, no NVIDIA driver, or a GPU from a vendor there is
    /// no portable way to ask.
    pub vram_mb: Option<u64>,
    /// One memory pool shared by CPU and GPU — Apple silicon. `cfg`-
    /// derived, not probed.
    pub unified_memory: bool,
}

impl Host {
    /// Read the live machine.
    ///
    /// Cheap but not free: on Linux and Windows it may spawn the NVIDIA
    /// driver's own tool. Call it off the foreground thread — a Settings
    /// window that blocks on a process spawn is a frozen window — and
    /// keep the answer; it does not change while the application runs.
    #[must_use]
    pub fn probe() -> Host {
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        let total_ram_mb = system.total_memory() / 1_048_576;
        let available_ram_mb = system.available_memory() / 1_048_576;
        let unified_memory = cfg!(target_os = "macos");
        Host {
            total_ram_mb,
            available_ram_mb,
            // On unified memory the system RAM *is* the ceiling the model
            // competes for, so it is the honest answer rather than a
            // missing one.
            vram_mb: if unified_memory {
                Some(total_ram_mb)
            } else {
                nvidia_vram_mb()
            },
            unified_memory,
        }
    }

    /// True when this machine is one the big model should not be the
    /// default on.
    ///
    /// Two independent conditions, either of which is enough: a discrete
    /// GPU of 12 GB or less, or a unified-memory machine of 16 GB or
    /// less. Deliberately **not** covered: a machine with plenty of RAM
    /// and no usable GPU at all. Handing that one a smaller model is not
    /// the fix, and inventing a rule for it would be guessing past what
    /// the numbers say.
    #[must_use]
    pub fn is_constrained(self) -> bool {
        let small_gpu = !self.unified_memory
            && self
                .vram_mb
                .is_some_and(|vram| vram <= CONSTRAINED_VRAM_MAX_MB);
        let small_unified =
            self.unified_memory && self.total_ram_mb <= CONSTRAINED_UNIFIED_RAM_MAX_MB;
        small_gpu || small_unified
    }
}

/// Whether an entry has room on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// There is room, with the machine's other work still able to run.
    Fits,
    /// It would fit and little else would. Offered, with the number said
    /// out loud, rather than hidden.
    Tight,
    /// More memory than this machine has.
    TooBig { short_by_mb: u64 },
    /// The probe could not read this machine. Never rendered as "no".
    Unknown,
}

impl Fit {
    /// True when a download is worth offering without a warning beside
    /// it.
    #[must_use]
    pub fn is_comfortable(self) -> bool {
        matches!(self, Fit::Fits)
    }
}

/// Is there room for `entry` on `host`? Pure.
///
/// Decided on RAM, not video memory: a machine with the RAM can always
/// run the model on its CPU. Video memory decides speed, and this
/// function does not claim to predict speed.
#[must_use]
pub fn fit(entry: &ModelEntry, host: Host) -> Fit {
    fit_mb(entry.mem.min_ram_mb, host)
}

/// [`fit`] for a model that needs `need` MiB — a model the person added,
/// whose figure is an estimate made from its header (E8-1) rather than
/// the catalogue's. The same rule, the same verdicts.
#[must_use]
pub fn fit_mb(need: u64, host: Host) -> Fit {
    if host.total_ram_mb == 0 {
        return Fit::Unknown;
    }
    if need > host.total_ram_mb {
        return Fit::TooBig {
            short_by_mb: need - host.total_ram_mb,
        };
    }
    if need * COMFORTABLE_SHARE > host.total_ram_mb * COMFORTABLE_OF {
        return Fit::Tight;
    }
    Fit::Fits
}

/// The memory a default is judged against on a constrained machine.
///
/// The video memory when it is known and separate — on a discrete
/// card that is the pool the model competes for, and the system RAM
/// beside it says nothing about whether the weights fit on the card —
/// and the RAM otherwise, which on unified memory *is* the pool.
fn pool(host: Host) -> u64 {
    if host.unified_memory {
        host.total_ram_mb
    } else {
        host.vram_mb.unwrap_or(host.total_ram_mb)
    }
}

/// Whether `entry` is one this machine should be pointed at by
/// default. Pure.
///
/// Two rules and the machine decides which applies. A roomy machine
/// takes anything [`fit`] calls comfortable. A constrained one — see
/// [`Host::is_constrained`] — also has to hold the entry within
/// [`CONSTRAINED_SHARE`] of its pool, which is the rule that keeps the
/// 12B off a 16 GB Mac while the 4B still qualifies. The RAM check is
/// kept underneath it, because a default that fits its card and not
/// its RAM is a default that does not run.
fn default_material(entry: &ModelEntry, host: Host) -> bool {
    if !fit(entry, host).is_comfortable() {
        return false;
    }
    !host.is_constrained() || entry.mem.min_ram_mb * CONSTRAINED_SHARE <= pool(host)
}

/// The entry to offer for `role` when the user has not chosen one.
///
/// The best-rated stable entry that has room, falling back to the
/// smallest one when nothing does — a machine too small for anything in
/// the catalogue is still shown the entry it is closest to affording,
/// beside the number it is short by, rather than an empty list.
/// `None` only when the catalogue has no stable entry for the role.
///
/// "Has room" is [`default_material`], which is stricter on a
/// constrained machine than [`fit`] is: the 12B *runs* on a 16 GB Mac
/// and is still the wrong thing to point that machine at, and the
/// smaller entry exists in the catalogue for exactly that machine.
#[must_use]
pub fn default_for_role(manifest: &Manifest, role: Role, host: Host) -> Option<&ModelEntry> {
    let candidates = manifest.for_role(role);
    let smallest = || {
        candidates
            .iter()
            .min_by_key(|entry| entry.mem.min_ram_mb)
            .copied()
    };
    if host.total_ram_mb == 0 {
        // The machine could not be read. The smallest entry is the one
        // choice that cannot be wrong for a reason we would have seen.
        return smallest();
    }
    candidates
        .iter()
        .find(|entry| default_material(entry, host))
        .or_else(|| {
            candidates
                .iter()
                .find(|entry| matches!(fit(entry, host), Fit::Tight))
        })
        .copied()
        .or_else(smallest)
}

/// Total device-local video memory of the first NVIDIA device, MiB.
///
/// Asked of `nvidia-smi`, which ships with the driver, rather than by
/// loading `libcuda`: this crate forbids unsafe code, and a desktop
/// application opening its Settings window can afford one process spawn
/// where a server accepting requests could not. `None` means unknown —
/// no driver, no tool on `PATH`, a query that failed, or a GPU from a
/// vendor with no equivalent — and every caller must read it that way.
///
/// Never run on macOS: there is no NVIDIA device to ask, and
/// [`Host::probe`] has a better answer there.
fn nvidia_vram_mb() -> Option<u64> {
    if cfg!(target_os = "macos") {
        return None;
    }
    let output = spawn_with_timeout(
        "nvidia-smi",
        &[
            "--query-gpu=memory.total",
            "--format=csv,noheader,nounits",
            "--id=0",
        ],
        PROBE_TIMEOUT,
    )?;
    let first = output.lines().next()?.trim();
    let mb = first.parse::<u64>().ok()?;
    (mb > 0).then_some(mb)
}

/// Run a command and read its stdout, giving up after `timeout`.
///
/// `std::process` has no timed wait, so the child is polled. A driver in
/// a bad state leaves `nvidia-smi` hanging, and the only thing worse
/// than not knowing how much video memory there is would be a Settings
/// window that never opens because it asked.
fn spawn_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    use std::process::{Command, Stdio};

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // Polling `try_wait` while the child writes to a pipe would deadlock
    // if the child produced more than the pipe buffer holds. It produces
    // one number and a newline, so it cannot — and nothing else is run
    // through here.
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => {
                return None;
            }
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                tracing::debug!(program, "probe did not answer in time; treating as unknown");
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Err(_) => return None,
        }
    }

    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        use std::io::Read;
        stdout.read_to_string(&mut out).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{
        default_for_role, fit, Fit, Host, CONSTRAINED_UNIFIED_RAM_MAX_MB, CONSTRAINED_VRAM_MAX_MB,
    };
    use crate::manifest::{Manifest, Role};

    fn mac(ram_mb: u64) -> Host {
        Host {
            total_ram_mb: ram_mb,
            available_ram_mb: ram_mb / 2,
            vram_mb: Some(ram_mb),
            unified_memory: true,
        }
    }

    fn pc(ram_mb: u64, vram_mb: Option<u64>) -> Host {
        Host {
            total_ram_mb: ram_mb,
            available_ram_mb: ram_mb / 2,
            vram_mb,
            unified_memory: false,
        }
    }

    /// The probe must not report a machine with no memory. Everything
    /// downstream reads zero as "could not tell", so a probe that
    /// silently returned it would turn every entry into `Unknown`.
    #[test]
    fn the_probe_reads_this_machine() {
        let host = Host::probe();
        assert!(host.total_ram_mb > 0, "sysinfo reported no memory at all");
        assert!(host.available_ram_mb <= host.total_ram_mb);
        assert_eq!(host.unified_memory, cfg!(target_os = "macos"));
        if cfg!(target_os = "macos") {
            assert_eq!(
                host.vram_mb,
                Some(host.total_ram_mb),
                "on unified memory the pool is the RAM"
            );
        }
    }

    /// An unknown GPU is not a small one. A machine with 64 GB of RAM
    /// and an AMD card there is no portable way to ask must not be
    /// handed the small model as though its card had been measured.
    #[test]
    fn an_unmeasured_gpu_is_not_a_small_gpu() {
        assert!(!pc(65_536, None).is_constrained());
        assert!(pc(65_536, Some(12_282)).is_constrained());
        assert!(!pc(65_536, Some(16_376)).is_constrained());
    }

    /// The two thresholds sit clear of the sizes cards and Macs actually
    /// report, which is the whole reason they are not round numbers.
    #[test]
    fn the_thresholds_separate_the_real_configurations() {
        for reported_12gb in [12_282, 12_288] {
            assert!(reported_12gb <= CONSTRAINED_VRAM_MAX_MB);
        }
        for reported_16gb_and_up in [16_376, 24_564] {
            assert!(reported_16gb_and_up > CONSTRAINED_VRAM_MAX_MB);
        }
        for reported_16gb_mac in [16_384_u64, 8_192] {
            assert!(reported_16gb_mac <= CONSTRAINED_UNIFIED_RAM_MAX_MB);
        }
        for reported_18gb_and_up in [18_432_u64, 36_864] {
            assert!(reported_18gb_and_up > CONSTRAINED_UNIFIED_RAM_MAX_MB);
        }
    }

    #[test]
    fn a_mac_is_judged_on_its_one_pool() {
        assert!(mac(16_384).is_constrained());
        assert!(!mac(36_864).is_constrained());
    }

    #[test]
    fn fit_answers_on_ram_and_says_how_short() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        let big = manifest
            .for_role(Role::Rewrite)
            .into_iter()
            .max_by_key(|entry| entry.mem.min_ram_mb)
            .expect("a rewriter");

        assert_eq!(fit(big, pc(0, None)), Fit::Unknown, "zero RAM is unknown");
        assert_eq!(fit(big, mac(131_072)), Fit::Fits);
        match fit(big, pc(1024, None)) {
            Fit::TooBig { short_by_mb } => {
                assert_eq!(short_by_mb, big.mem.min_ram_mb - 1024);
            }
            other => panic!("a 1 GB machine should not fit a 12B model: {other:?}"),
        }
        // Exactly enough RAM and nothing spare is offered, not hidden —
        // with the tight verdict that lets a surface say so.
        assert_eq!(fit(big, pc(big.mem.min_ram_mb, None)), Fit::Tight);
        assert!(!Fit::Tight.is_comfortable());
        assert!(Fit::Fits.is_comfortable());
    }

    /// The whole point of shipping two rewriters: a roomy machine is
    /// offered the larger one and a 12 GB machine is not.
    #[test]
    fn a_small_machine_is_offered_the_small_model() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        let roomy = default_for_role(&manifest, Role::Rewrite, mac(131_072)).expect("a default");
        let tiny =
            default_for_role(&manifest, Role::Rewrite, pc(8_192, Some(8_192))).expect("a default");
        assert!(
            roomy.mem.min_ram_mb >= tiny.mem.min_ram_mb,
            "the constrained machine was handed the larger model"
        );
        assert_ne!(
            roomy.id, tiny.id,
            "the catalogue must offer something smaller than its best entry"
        );
    }

    /// A machine too small for anything is still shown an entry — the
    /// one it is closest to affording — rather than an empty list with
    /// no explanation.
    #[test]
    fn a_machine_too_small_for_everything_is_still_offered_something() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        let entry = default_for_role(&manifest, Role::Rewrite, pc(512, None))
            .expect("something is always offered");
        assert!(matches!(fit(entry, pc(512, None)), Fit::TooBig { .. }));
        let smallest = manifest
            .for_role(Role::Rewrite)
            .into_iter()
            .min_by_key(|e| e.mem.min_ram_mb)
            .expect("a rewriter");
        assert_eq!(entry.id, smallest.id);
    }

    /// The rule this module took from `mnemoria-lvkb` and, until now,
    /// applied to nothing: on a machine [`Host::is_constrained`] names,
    /// the default is not the best entry that would *run* but the best
    /// one that claims no more than half the pool it competes for. A
    /// 16 GB Mac holds the 12B by the RAM figure alone — 9216 of 16384
    /// is under three quarters — and that is exactly the machine the
    /// smaller entry was shipped for.
    #[test]
    fn a_constrained_machine_is_offered_the_small_model() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        let smallest = manifest
            .for_role(Role::Rewrite)
            .into_iter()
            .min_by_key(|entry| entry.mem.min_ram_mb)
            .expect("a rewriter");
        // A 16 GB Mac, and a 12 GB card in a box with plenty of RAM:
        // the two configurations the thresholds exist for.
        for host in [mac(16_384), pc(65_536, Some(12_282))] {
            assert!(host.is_constrained(), "{host:?} is the constrained case");
            let offered = default_for_role(&manifest, Role::Rewrite, host).expect("a default");
            assert_eq!(
                offered.id, smallest.id,
                "{host:?} was handed {} rather than the entry shipped for it",
                offered.id
            );
        }
        // And the next configuration up on each side is not constrained
        // and gets the best entry it has room for — what `default_for_role`
        // means. Since E8-1 that is not the catalogue's best on both: an
        // 18 GB Mac holds Qwen3.8 27B only tightly, and is offered the best
        // entry below it; a box with 64 GB beside its 16 GB card holds it
        // comfortably. Either way something larger than the entry shipped
        // for the constrained machines.
        for host in [mac(18_432), pc(65_536, Some(16_376))] {
            assert!(!host.is_constrained(), "{host:?} is the roomy case");
            let roomiest = manifest
                .for_role(Role::Rewrite)
                .into_iter()
                .find(|entry| fit(entry, host).is_comfortable())
                .expect("an entry this machine has room for");
            let offered = default_for_role(&manifest, Role::Rewrite, host).expect("a default");
            assert_eq!(
                offered.id, roomiest.id,
                "{host:?} should be offered the best entry it has room for"
            );
            assert_ne!(
                offered.id, smallest.id,
                "{host:?} was handed the entry shipped for the constrained machines"
            );
        }
        let best = manifest
            .for_role(Role::Rewrite)
            .into_iter()
            .max_by_key(|entry| entry.quality_tier)
            .expect("a rewriter");
        assert_eq!(
            default_for_role(&manifest, Role::Rewrite, pc(65_536, Some(16_376)))
                .expect("a default")
                .id,
            best.id,
            "a machine with room for everything is offered the catalogue's best"
        );
        assert_ne!(
            default_for_role(&manifest, Role::Rewrite, mac(18_432))
                .expect("a default")
                .id,
            best.id,
            "an 18 GB Mac holds the best entry only tightly"
        );
    }

    /// No entry serves it, so nothing is invented for it.
    #[test]
    fn a_role_with_no_entry_has_no_default() {
        let manifest = Manifest::embedded().expect("embedded manifest");
        assert!(default_for_role(&manifest, Role::Pixel, mac(131_072)).is_none());
    }
}
