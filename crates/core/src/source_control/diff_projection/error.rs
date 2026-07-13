//! Diff projection resource and cancellation errors.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract

use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DiffProjectionError {
    #[error("diff input bytes {actual} exceed limit {limit}")]
    InputBytes { actual: usize, limit: usize },
    #[error("diff input lines {actual} exceed limit {limit}")]
    InputLines { actual: usize, limit: usize },
    #[error("diff projection bytes {actual} exceed limit {limit}")]
    OutputBytes { actual: usize, limit: usize },
    #[error("diff projection was cancelled")]
    Cancelled,
    #[error("diff projection computation exceeded its bounded deadline")]
    ComputeDeadline,
    #[error("diff projection invariant failed: {0}")]
    Invariant(&'static str),
}

impl DiffProjectionError {
    pub fn is_resource_limit(&self) -> bool {
        matches!(
            self,
            Self::InputBytes { .. } | Self::InputLines { .. } | Self::OutputBytes { .. }
        )
    }
}
