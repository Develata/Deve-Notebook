//! plan_ref:
//!   - 16_web_thin_client_ledger#web-edit-intent
//!   - 15_release#runtime-observability
//!
use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::cannot_action;

pub(super) fn allow_repo_write(core: &CoreState, action: &'static str) -> bool {
    let block = repo_write_block_untracked(
        &core.ws,
        RepoWriteSignals {
            load_state: core.load_state,
            is_spectator: core.is_spectator,
            handshake_ready: core.handshake_ready,
            current_repo_id: core.current_repo_id,
            active_branch: core.active_branch,
            pending_branch_switch: core.pending_branch_switch,
            pending_repo_switch: core.pending_repo_switch,
        },
    );
    let Some(block) = block else {
        return true;
    };
    let message = cannot_action(action, block.label());
    warn_sync_banner(core.set_sync_banner, message);
    false
}
