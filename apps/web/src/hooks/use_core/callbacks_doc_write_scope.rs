use crate::api::WsService;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::cannot_send;
use leptos::prelude::WriteSignal;

pub(super) fn local_write_scope_nonce(
    ws: &WsService,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    action: &'static str,
) -> Option<u64> {
    if let Some(block) = repo_write_block_untracked(ws, write_gate) {
        let message = cannot_send(action, block.label());
        warn_sync_banner(set_sync_banner, message);
        return None;
    }
    let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
        let message = cannot_send(action, "local repo scope is not stable");
        warn_sync_banner(set_sync_banner, message);
        return None;
    };
    Some(scope_nonce)
}
