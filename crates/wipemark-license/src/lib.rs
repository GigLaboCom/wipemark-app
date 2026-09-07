//! `wipemark-license` — activation, grace and what a lapsed licence
//! actually locks.
//!
//! Implements the shared Heretic scheme (`heretic-license-activation-spec`):
//! Ed25519 / PASETO v4.public tokens, a device fingerprint, and the
//! token stored in the OS keychain rather than in a file.
//!
//! # What a lapsed licence must never lock
//!
//! Layer A — the deterministic Unicode scrubber — stays available
//! always, licensed or not, online or not (spec §8). It is the part of
//! the product a user can verify for themselves, it costs us nothing to
//! run, and holding it hostage after an expired token would make the
//! tool untrustworthy exactly when someone needs it. Only Layer B
//! (model rewriting) is gated.
//!
//! # Skeleton status
//!
//! Epic **E0**: the states only. Verification lands in epic E9.

#![forbid(unsafe_code)]

use std::time::Duration;

/// What the current install is entitled to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseState {
    /// No licence yet. Layer A unlimited, Layer B on trial terms.
    Trial { remaining: TrialAllowance },
    /// Verified token, online check within the window.
    Active,
    /// Verified token, last successful check inside the offline grace
    /// window. Fully functional — a laptop on a plane is not a pirate.
    OfflineGrace { remaining: Duration },
    /// Grace exhausted. Layer B is locked; Layer A is not.
    Lapsed,
    /// The token did not verify at all.
    Invalid { reason: String },
}

/// The trial's shape. Which of the two it is remains an owner decision
/// (spec §11, Q6) — the type carries both so the decision does not
/// require a refactor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialAllowance {
    /// N Layer B documents per day, forever.
    DocumentsPerDay { used: u32, limit: u32 },
    /// Unlimited Layer B for a fixed number of days.
    Days { remaining: u32 },
}

impl LicenseState {
    /// Layer A is available in every state. This is a product promise,
    /// so it is a function rather than a comment: a future refactor that
    /// wants to gate it has to delete this body and fail the test below,
    /// rather than quietly add a condition somewhere.
    #[allow(
        clippy::unused_self,
        reason = "the promise is that no state changes this answer"
    )]
    pub fn layer_a_available(&self) -> bool {
        true
    }

    /// Whether a rewrite may start right now.
    pub fn layer_b_available(&self) -> bool {
        match self {
            LicenseState::Active | LicenseState::OfflineGrace { .. } => true,
            LicenseState::Trial { remaining } => remaining.has_headroom(),
            LicenseState::Lapsed | LicenseState::Invalid { .. } => false,
        }
    }
}

impl TrialAllowance {
    pub fn has_headroom(&self) -> bool {
        match self {
            TrialAllowance::DocumentsPerDay { used, limit } => used < limit,
            TrialAllowance::Days { remaining } => *remaining > 0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("token signature did not verify")]
    BadSignature,
    #[error("token is for a different device")]
    WrongDevice,
    #[error("keychain: {0}")]
    Keychain(String),
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{LicenseState, TrialAllowance};

    /// The one guarantee in this crate that is a promise to users rather
    /// than an implementation detail.
    #[test]
    fn layer_a_survives_every_licence_state() {
        let states = [
            LicenseState::Active,
            LicenseState::Lapsed,
            LicenseState::OfflineGrace {
                remaining: Duration::from_secs(0),
            },
            LicenseState::Invalid {
                reason: "expired".to_owned(),
            },
            LicenseState::Trial {
                remaining: TrialAllowance::Days { remaining: 0 },
            },
        ];
        for state in states {
            assert!(state.layer_a_available(), "{state:?} locked Layer A");
        }
    }

    #[test]
    fn lapsed_locks_only_layer_b() {
        assert!(!LicenseState::Lapsed.layer_b_available());
        assert!(LicenseState::OfflineGrace {
            remaining: Duration::from_secs(3600)
        }
        .layer_b_available());
    }

    #[test]
    fn exhausted_trial_stops_rewriting() {
        let spent = LicenseState::Trial {
            remaining: TrialAllowance::DocumentsPerDay { used: 5, limit: 5 },
        };
        assert!(!spent.layer_b_available());
    }
}
