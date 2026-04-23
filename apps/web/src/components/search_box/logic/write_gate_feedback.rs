use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use leptos::prelude::Set;

#[cfg(test)]
#[path = "write_gate_feedback_test.rs"]
mod tests;

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
    let message = write_block_message(action, block.label());
    leptos::logging::warn!("{}", message);
    core.set_sync_banner.set(Some(message));
    false
}

fn write_block_message(action: &str, reason: &str) -> String {
    format!("Cannot {}: {}", action, reason)
}
