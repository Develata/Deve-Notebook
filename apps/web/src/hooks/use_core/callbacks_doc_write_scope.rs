use crate::api::WsService;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use leptos::prelude::{Set, WriteSignal};

#[cfg(test)]
#[path = "callbacks_doc_write_scope_test.rs"]
mod tests;

pub(super) fn local_write_scope_nonce(
    ws: &WsService,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    action: &'static str,
) -> Option<u64> {
    if let Some(block) = repo_write_block_untracked(ws, write_gate) {
        let message = write_block_banner(action, block.label());
        leptos::logging::warn!("{}", message);
        set_sync_banner.set(Some(message));
        return None;
    }
    let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
        let message = write_block_banner(action, "local repo scope is not stable");
        leptos::logging::warn!("{}", message);
        set_sync_banner.set(Some(message));
        return None;
    };
    Some(scope_nonce)
}

fn write_block_banner(action: &str, reason: &str) -> String {
    format!("Cannot send {}: {}", action, reason)
}
