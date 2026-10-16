//! How far a load of weights has got (F1).
//!
//! A load is seconds to tens of seconds — Qwen3.8 27B reads in 4.6–21 s
//! from the page cache — and llama.cpp reports the fraction read once per
//! tensor: hundreds of reports for a small model, more for a large one.
//! An engine that loads tells a [`LoadSink`] what it read, through a
//! [`Pacer`] that lets at most one report through every [`Pacer::EVERY`]
//! (and always the first and the last), and says [`LoadProgress::Ended`]
//! when the load is over, however it ended — so a bar never stays on
//! screen over a load that was refused, stopped or finished.

use std::time::{Duration, Instant};

/// One report from a load.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadProgress {
    /// The fraction of the weights read so far, from 0.0 to 1.0.
    Reading(f32),
    /// What became of the speculative draft asked to load beside the model
    /// (E2-dflash2): `Ok` when it decodes beside it, the refusal when the
    /// model was loaded alone. Told once, before [`LoadProgress::Ended`],
    /// by a load that asked for a draft — and only by one.
    Draft(Result<(), crate::DraftRefusal>),
    /// The load is over: loaded, refused or stopped. Nothing more is
    /// read until the next load says [`LoadProgress::Reading`] again.
    Ended,
}

/// Where an engine tells how its loads are going: a `flume` sender, for
/// the reason [`crate::TokenSink`] is one — the receiving end is the GPUI
/// executor — and the number the receiver handed out with it, sent with
/// every report. One receiver can listen to engine after engine and tell a
/// report from one it let go of — an abandoned load's `Ended` arriving
/// after the next engine started — from the current one's (L6). A send to
/// a receiver that went away is not an error.
#[derive(Debug, Clone)]
pub struct LoadSink {
    to: flume::Sender<(u64, LoadProgress)>,
    of: u64,
}

impl LoadSink {
    /// Reports go to `to`, each with `of`.
    pub fn new(to: flume::Sender<(u64, LoadProgress)>, of: u64) -> Self {
        Self { to, of }
    }

    /// Tell one report. `false` once nobody listens.
    pub fn send(&self, told: LoadProgress) -> bool {
        self.to.send((self.of, told)).is_ok()
    }
}

/// The rate limit between llama.cpp's per-tensor reports and a sink.
#[derive(Debug, Default)]
pub struct Pacer {
    /// When the last report went through, and what it said.
    last: Option<(Instant, f32)>,
}

impl Pacer {
    /// At most one report this often: ten a second is a bar that moves
    /// smoothly, and a window repainted for nothing else.
    pub const EVERY: Duration = Duration::from_millis(100);

    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the report of `fraction` at `now` goes through: the first
    /// always, the end of the read (1.0) always, and otherwise one that
    /// comes [`Pacer::EVERY`] after the last that went and says more than
    /// it did. A fraction is clamped to 0..=1 and a NaN is never told.
    pub fn admit(&mut self, now: Instant, fraction: f32) -> Option<f32> {
        if fraction.is_nan() {
            return None;
        }
        let fraction = fraction.clamp(0.0, 1.0);
        let through = match self.last {
            None => true,
            Some((_, said)) if fraction >= 1.0 => said < 1.0,
            Some((at, said)) => fraction > said && now.duration_since(at) >= Self::EVERY,
        };
        if through {
            self.last = Some((now, fraction));
            Some(fraction)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Pacer;

    /// A load is hundreds of reports in a moment; what goes through is the
    /// first, one per tenth of a second, and the last — never a report
    /// per tensor.
    #[test]
    fn a_report_per_tensor_is_paced() {
        let start = Instant::now();
        let mut pacer = Pacer::new();
        let mut told = Vec::new();
        // 1000 tensors over half a second.
        for tensor in 0..=1000u32 {
            let now = start + Duration::from_micros(u64::from(tensor) * 500);
            if let Some(fraction) = pacer.admit(now, tensor as f32 / 1000.0) {
                told.push(fraction);
            }
        }
        assert_eq!(told.first(), Some(&0.0));
        assert_eq!(told.last(), Some(&1.0), "the end of the read was not told");
        assert!(
            told.len() <= 8,
            "{} reports went through for half a second: {told:?}",
            told.len()
        );
        assert!(
            told.windows(2).all(|pair| pair[0] < pair[1]),
            "the bar went backwards: {told:?}"
        );
    }

    /// The end is told once, a fraction out of range is clamped, and a NaN
    /// is never told.
    #[test]
    fn the_end_is_told_once_and_nonsense_never() {
        let start = Instant::now();
        let mut pacer = Pacer::new();
        assert_eq!(pacer.admit(start, f32::NAN), None);
        assert_eq!(pacer.admit(start, -0.5), Some(0.0));
        assert_eq!(pacer.admit(start, 1.5), Some(1.0));
        assert_eq!(pacer.admit(start + Duration::from_secs(1), 1.0), None);
    }
}
