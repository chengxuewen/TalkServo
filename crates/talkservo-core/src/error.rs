//! Core-domain error type (modules/05, PoC slice).
//!
//! The wire contract surfaces failures as `SignalingMessage::Error` — this
//! enum is for in-crate domain failures (state construction, apply() misuse),
//! not for wire reporting.

use thiserror::Error;

/// Domain-level failure inside the floor model.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// A constructor received a generation of `u64::MAX` — reserved sentinel.
    #[error("generation {0} is reserved and cannot be used")]
    ReservedGeneration(u64),
    /// FloorLimits invariant violated (max_queue must be >= max nonzero grants capacity).
    #[error("invalid floor limits: max_peers={max_peers}, max_queue={max_queue}")]
    InvalidLimits {
        max_peers: usize,
        max_queue: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_stable() {
        assert_eq!(
            CoreError::ReservedGeneration(u64::MAX).to_string(),
            "generation 18446744073709551615 is reserved and cannot be used"
        );
        assert_eq!(
            CoreError::InvalidLimits {
                max_peers: 0,
                max_queue: 4
            }
            .to_string(),
            "invalid floor limits: max_peers=0, max_queue=4"
        );
    }
}
