//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 05_diff_logic#source-control-runtime
//!
//! Typed commit-authority phases for server-side mutation publication.

use super::ExternalApplyReceipt;

/// Failure boundary for the authority-only part of a Source Control commit.
///
/// Resolved-conflict commits may first apply staged external facts and then
/// fail before writing the commit anchor. Callers must publish recovery for
/// that committed prefix instead of reporting the whole operation as
/// uncommitted.
#[derive(Debug)]
pub enum CommitAuthorityFailure {
    NotCommitted(anyhow::Error),
    CommittedPartial {
        external_apply: ExternalApplyReceipt,
        error: anyhow::Error,
    },
}

impl CommitAuthorityFailure {
    pub fn into_error(self) -> anyhow::Error {
        match self {
            Self::NotCommitted(error) | Self::CommittedPartial { error, .. } => error,
        }
    }

    pub fn error(&self) -> &anyhow::Error {
        match self {
            Self::NotCommitted(error) | Self::CommittedPartial { error, .. } => error,
        }
    }
}
