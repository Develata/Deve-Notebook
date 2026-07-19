//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator

use super::CatalogMembershipError;
use crate::models::RepoId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoCatalogError {
    #[error("repo catalog I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("repo catalog JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("repo catalog membership failed: {0}")]
    Membership(#[from] CatalogMembershipError),
    #[error("repo catalog record for {0} does not exist")]
    NotFound(RepoId),
    #[error("repo catalog record for {0} already exists")]
    AlreadyExists(RepoId),
    #[error("repo catalog authority is busy in another process")]
    AuthorityBusy,
    #[error("repo catalog record for {repo_id} is {actual}, expected {expected}")]
    StateMismatch {
        repo_id: RepoId,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("repo catalog record identity mismatch: expected {expected}, got {actual}")]
    RecordIdentityMismatch { expected: RepoId, actual: RepoId },
    #[error("repo catalog record is invalid: {0}")]
    InvalidRecord(String),
    #[error("repo catalog prepared identity changed for {0}")]
    PreparedIdentityChanged(RepoId),
    #[error("repo catalog prepared identity could not be established for {repo_id}: {detail}")]
    PreparedIdentityUnavailable { repo_id: RepoId, detail: String },
    #[error("repo catalog membership revision exhausted for {0}")]
    MembershipRevisionExhausted(RepoId),
    #[error("repo catalog publish for {repo_id} failed at {phase}: {primary}; cleanup={cleanup:?}")]
    PublishFailed {
        repo_id: RepoId,
        phase: &'static str,
        primary: String,
        cleanup: Option<String>,
    },
    #[error(
        "repo catalog cut for {repo_id} committed durably but process membership failed: {detail}"
    )]
    DurableCutProcessStateFailed { repo_id: RepoId, detail: String },
    #[error("repo catalog cut for {repo_id} has unknown durable outcome: {detail}")]
    CutOutcomeUnknown { repo_id: RepoId, detail: String },
    #[error("repo catalog runtime lock is poisoned")]
    Poisoned,
}

pub(super) fn state_name(state: super::RepoCatalogMembershipState) -> &'static str {
    match state {
        super::RepoCatalogMembershipState::Normal => "normal",
        super::RepoCatalogMembershipState::Removed => "removed",
    }
}
