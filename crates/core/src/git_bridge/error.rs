//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle

mod command;
mod import;
mod preflight_replay;
mod run;
mod status_store;

pub(super) use command::{GitBridgeError, GitCommandError};
pub use import::{GitImportApplyError, GitImportPlanError};
pub(super) use preflight_replay::{
    GitMirrorCommitError, GitPreflightError, GitProjectionReplayError, GitReplayError,
    GitReplayPlanError, GitSnapshotBootstrapError,
};
pub(super) use run::GitMirrorRunFailure;
pub use run::{GitMirrorPushError, GitMirrorRunError};
pub use status_store::{GitMirrorStatusError, GitMirrorStoreError};

pub(super) type GitBridgeResult<T> = std::result::Result<T, GitBridgeError>;
pub(super) type GitCommandResult<T> = std::result::Result<T, GitCommandError>;
pub(super) type GitPreflightResult<T> = std::result::Result<T, GitPreflightError>;
pub(super) type GitReplayResult<T> = std::result::Result<T, GitReplayError>;
pub(super) type GitReplayPlanResult<T> = std::result::Result<T, GitReplayPlanError>;
pub(super) type GitProjectionReplayResult<T> = std::result::Result<T, GitProjectionReplayError>;
pub(super) type GitMirrorCommitResult<T> = std::result::Result<T, GitMirrorCommitError>;
pub(super) type GitSnapshotBootstrapResult<T> = std::result::Result<T, GitSnapshotBootstrapError>;
pub type GitMirrorStatusResult<T> = std::result::Result<T, GitMirrorStatusError>;
pub type GitImportPlanResult<T> = std::result::Result<T, GitImportPlanError>;
pub type GitImportApplyResult<T> = std::result::Result<T, GitImportApplyError>;
pub type GitMirrorRunResult<T> = std::result::Result<T, GitMirrorRunError>;
pub type GitMirrorPushResult<T> = std::result::Result<T, GitMirrorPushError>;
pub type GitMirrorStoreResult<T> = std::result::Result<T, GitMirrorStoreError>;

#[cfg(test)]
mod failure_tests;
#[cfg(test)]
mod tests;
