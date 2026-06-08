//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 18_release#runtime-observability
//!
use crate::components::search_box::runtime::SearchRuntime;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::cannot_action;

pub(super) fn allow_repo_write(runtime: &SearchRuntime, action: &'static str) -> bool {
    let block = repo_write_block_untracked(
        &runtime.session.ws,
        RepoWriteSignals {
            load_state: runtime.editor.load_state,
            is_spectator: runtime.editor.is_spectator,
            handshake_ready: runtime.editor.handshake_ready,
            current_repo_id: runtime.editor.current_repo_id,
            current_scope_nonce: runtime.editor.current_scope_nonce,
            active_branch: runtime.editor.active_branch,
            pending_branch_switch: runtime.editor.pending_branch_switch,
            pending_repo_switch: runtime.editor.pending_repo_switch,
        },
    );
    let Some(block) = block else {
        return true;
    };
    let message = cannot_action(action, block.label());
    warn_sync_banner(runtime.session.set_sync_banner, message);
    false
}
