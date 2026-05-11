//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
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
        show_doc_write_block(set_sync_banner, action, block.label());
        return None;
    }
    let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
        show_doc_write_block(set_sync_banner, action, "local repo scope is not stable");
        return None;
    };
    Some(scope_nonce)
}

fn show_doc_write_block(set_sync_banner: WriteSignal<Option<String>>, action: &str, reason: &str) {
    let message = cannot_send(action, reason);
    warn_sync_banner(set_sync_banner, message);
}

#[cfg(test)]
mod tests {
    use super::show_doc_write_block;
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn doc_write_block_banner_includes_action_and_reason() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);

        show_doc_write_block(set_sync_banner, "MoveDoc", "local repo scope is not stable");

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("Cannot send MoveDoc: local repo scope is not stable")
        );
    }
}
