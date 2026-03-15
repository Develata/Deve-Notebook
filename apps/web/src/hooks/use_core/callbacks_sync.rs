use crate::api::WsService;
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};

pub struct SyncCallbacks {
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    pub on_list_shadows: Callback<()>,
    pub on_merge_peer: Callback<String>,
}

pub fn create_sync_callbacks(
    ws: &WsService,
    current_doc: ReadSignal<Option<DocId>>,
    local_scope: LocalScopeSignals,
    set_shadow_list_request_id: WriteSignal<Option<String>>,
    set_sync_mode_request_id: WriteSignal<Option<String>>,
    set_pending_ops_request_id: WriteSignal<Option<String>>,
) -> SyncCallbacks {
    let ws1 = ws.clone();
    let on_get_sync_mode = Callback::new(move |_: ()| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 GetSyncMode: local repo scope 尚未稳定");
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_sync_mode_request_id.set(Some(request_id.clone()));
        ws1.send(ClientMessage::GetSyncMode {
            request_id,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws2 = ws.clone();
    let on_set_sync_mode = Callback::new(move |mode: String| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 SetSyncMode: local repo scope 尚未稳定");
            return;
        };
        ws2.send(ClientMessage::SetSyncMode {
            mode,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws3 = ws.clone();
    let on_get_pending_ops = Callback::new(move |_: ()| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 GetPendingOps: local repo scope 尚未稳定");
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_pending_ops_request_id.set(Some(request_id.clone()));
        ws3.send(ClientMessage::GetPendingOps {
            request_id,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws4 = ws.clone();
    let on_confirm_merge = Callback::new(move |_: ()| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 ConfirmMerge: local repo scope 尚未稳定");
            return;
        };
        ws4.send(ClientMessage::ConfirmMerge {
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws5 = ws.clone();
    let on_discard_pending = Callback::new(move |_: ()| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 DiscardPending: local repo scope 尚未稳定");
            return;
        };
        ws5.send(ClientMessage::DiscardPending {
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws6 = ws.clone();
    let on_list_shadows = Callback::new(move |_: ()| {
        let request_id = uuid::Uuid::new_v4().to_string();
        set_shadow_list_request_id.set(Some(request_id.clone()));
        ws6.send(ClientMessage::ListShadows { request_id });
    });

    let ws7 = ws.clone();
    let on_merge_peer = Callback::new(move |peer_id: String| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 MergePeer: local repo scope 尚未稳定");
            return;
        };
        if let Some(doc_id) = current_doc.get_untracked() {
            ws7.send(ClientMessage::MergePeer {
                peer_id,
                doc_id,
                scope_nonce: Some(scope_nonce),
            });
        }
    });

    SyncCallbacks {
        on_get_sync_mode,
        on_set_sync_mode,
        on_get_pending_ops,
        on_confirm_merge,
        on_discard_pending,
        on_list_shadows,
        on_merge_peer,
    }
}
