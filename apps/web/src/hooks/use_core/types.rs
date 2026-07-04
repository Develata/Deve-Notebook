//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
#![allow(dead_code)]

mod core_state;
mod core_state_chat;
mod shared;

pub use crate::runtime::domain::{
    AiBackendMode, LoadPhase, PendingBranchSwitch, PendingBranchTarget, PendingOpsPreview,
    PendingRepoSwitch, SearchHit, SyncModeState,
};
pub use core_state::CoreState;
pub use shared::{
    ChatMessage, HandshakeSignals, PeerSession, RepoSwitchSignals, SwitchScopeSignals,
};
