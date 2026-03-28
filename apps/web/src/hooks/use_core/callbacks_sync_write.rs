use crate::api::WsService;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};

pub(super) struct SyncWriteCallbacks {
    pub(super) on_set_sync_mode: Callback<String>,
    pub(super) on_confirm_merge: Callback<()>,
    pub(super) on_discard_pending: Callback<()>,
    pub(super) on_merge_peer: Callback<String>,
}

pub(super) fn create_sync_write_callbacks(
    ws: &WsService,
    current_doc: ReadSignal<Option<DocId>>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
) -> SyncWriteCallbacks {
    let ws1 = ws.clone();
    let on_set_sync_mode = Callback::new(move |mode: String| {
        if let Some(block) = repo_write_block_untracked(&ws1, write_gate) {
            leptos::logging::warn!("忽略 SetSyncMode: {}", block.label());
            return;
        }
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 SetSyncMode: local repo scope 尚未稳定");
            return;
        };
        ws1.send(ClientMessage::SetSyncMode {
            mode,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws2 = ws.clone();
    let on_confirm_merge = Callback::new(move |_: ()| {
        if let Some(block) = repo_write_block_untracked(&ws2, write_gate) {
            leptos::logging::warn!("忽略 ConfirmMerge: {}", block.label());
            return;
        }
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 ConfirmMerge: local repo scope 尚未稳定");
            return;
        };
        ws2.send(ClientMessage::ConfirmMerge {
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws3 = ws.clone();
    let on_discard_pending = Callback::new(move |_: ()| {
        if let Some(block) = repo_write_block_untracked(&ws3, write_gate) {
            leptos::logging::warn!("忽略 DiscardPending: {}", block.label());
            return;
        }
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 DiscardPending: local repo scope 尚未稳定");
            return;
        };
        ws3.send(ClientMessage::DiscardPending {
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws4 = ws.clone();
    let on_merge_peer = Callback::new(move |peer_id: String| {
        if let Some(block) = repo_write_block_untracked(&ws4, write_gate) {
            leptos::logging::warn!("忽略 MergePeer: {}", block.label());
            return;
        }
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 MergePeer: local repo scope 尚未稳定");
            return;
        };
        if let Some(doc_id) = current_doc.get_untracked() {
            ws4.send(ClientMessage::MergePeer {
                peer_id,
                doc_id,
                scope_nonce: Some(scope_nonce),
            });
        }
    });

    SyncWriteCallbacks {
        on_set_sync_mode,
        on_confirm_merge,
        on_discard_pending,
        on_merge_peer,
    }
}
