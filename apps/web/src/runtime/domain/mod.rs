//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! Shared Web runtime domain contract.
//!
//! These types are consumed by `runtime/*_client` and re-exported by
//! `hooks/use_core` while it remains the application-control composition root.

mod runtime_state;
mod scope;

pub use runtime_state::{AiBackendMode, LoadPhase, PendingOpsPreview, SearchHit, SyncModeState};
pub use scope::{
    PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch, RepoRemoveRequest,
    RepoRenameRequest, RepoSwitchRequest,
};
