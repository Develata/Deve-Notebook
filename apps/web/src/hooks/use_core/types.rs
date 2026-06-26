//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
#![allow(dead_code)]

mod core_state;
mod core_state_chat;
mod pending_switch;
mod runtime_state;
mod shared;

pub use core_state::CoreState;
pub use pending_switch::{PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch};
pub use runtime_state::{AiBackendMode, LoadPhase, PendingOpsPreview, SearchHit, SyncModeState};
pub use shared::{
    ChatMessage, HandshakeSignals, PeerSession, RepoSwitchSignals, SwitchScopeSignals,
};
