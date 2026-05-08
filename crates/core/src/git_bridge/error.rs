//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle

pub(super) type GitBridgeResult<T> = std::result::Result<T, GitBridgeError>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum GitBridgeError {
    #[error("Git push mirror refuses invalid {label}: {value:?}")]
    InvalidPushName { label: &'static str, value: String },
    #[error("Git push mirror requires a named branch; detached HEAD needs --branch")]
    DetachedHead,
    #[error("{0}")]
    GitCommand(String),
}
