use crate::api::WsService;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};

pub(super) fn local_write_scope_nonce(
    ws: &WsService,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    action: &'static str,
) -> Option<u64> {
    if let Some(block) = repo_write_block_untracked(ws, write_gate) {
        leptos::logging::warn!("忽略 {}: {}", action, block.label());
        return None;
    }
    let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
        leptos::logging::warn!("忽略 {}: local repo scope 尚未稳定", action);
        return None;
    };
    Some(scope_nonce)
}
