use crate::api::WsService;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::cannot_send;
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
    set_sync_banner: WriteSignal<Option<String>>,
) -> SyncWriteCallbacks {
    let ws1 = ws.clone();
    let on_set_sync_mode = Callback::new(move |mode: String| {
        let Some(scope_nonce) = sync_write_scope_nonce(
            &ws1,
            local_scope,
            write_gate,
            set_sync_banner,
            "SetSyncMode",
        ) else {
            return;
        };
        ws1.send(ClientMessage::SetSyncMode {
            mode,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws2 = ws.clone();
    let on_confirm_merge = Callback::new(move |_: ()| {
        let Some(scope_nonce) = sync_write_scope_nonce(
            &ws2,
            local_scope,
            write_gate,
            set_sync_banner,
            "ConfirmMerge",
        ) else {
            return;
        };
        ws2.send(ClientMessage::ConfirmMerge {
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws3 = ws.clone();
    let on_discard_pending = Callback::new(move |_: ()| {
        let Some(scope_nonce) = sync_write_scope_nonce(
            &ws3,
            local_scope,
            write_gate,
            set_sync_banner,
            "DiscardPending",
        ) else {
            return;
        };
        ws3.send(ClientMessage::DiscardPending {
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws4 = ws.clone();
    let on_merge_peer = Callback::new(move |peer_id: String| {
        let Some(scope_nonce) =
            sync_write_scope_nonce(&ws4, local_scope, write_gate, set_sync_banner, "MergePeer")
        else {
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

fn sync_write_scope_nonce(
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
