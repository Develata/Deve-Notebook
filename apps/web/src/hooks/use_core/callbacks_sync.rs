use crate::api::WsService;
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::callbacks_scope::{LocalScopeSignals, run_if_stable_local_scope};

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
        let ws = ws1.clone();
        run_if_stable_local_scope(local_scope, "GetSyncMode", move || {
            let request_id = uuid::Uuid::new_v4().to_string();
            set_sync_mode_request_id.set(Some(request_id.clone()));
            ws.send(ClientMessage::GetSyncMode { request_id });
        });
    });

    let ws2 = ws.clone();
    let on_set_sync_mode = Callback::new(move |mode: String| {
        let ws = ws2.clone();
        run_if_stable_local_scope(local_scope, "SetSyncMode", move || {
            ws.send(ClientMessage::SetSyncMode { mode });
        });
    });

    let ws3 = ws.clone();
    let on_get_pending_ops = Callback::new(move |_: ()| {
        let ws = ws3.clone();
        run_if_stable_local_scope(local_scope, "GetPendingOps", move || {
            let request_id = uuid::Uuid::new_v4().to_string();
            set_pending_ops_request_id.set(Some(request_id.clone()));
            ws.send(ClientMessage::GetPendingOps { request_id });
        });
    });

    let ws4 = ws.clone();
    let on_confirm_merge = Callback::new(move |_: ()| {
        let ws = ws4.clone();
        run_if_stable_local_scope(local_scope, "ConfirmMerge", move || {
            ws.send(ClientMessage::ConfirmMerge);
        });
    });

    let ws5 = ws.clone();
    let on_discard_pending = Callback::new(move |_: ()| {
        let ws = ws5.clone();
        run_if_stable_local_scope(local_scope, "DiscardPending", move || {
            ws.send(ClientMessage::DiscardPending);
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
        let ws = ws7.clone();
        run_if_stable_local_scope(local_scope, "MergePeer", move || {
            if let Some(doc_id) = current_doc.get_untracked() {
                ws.send(ClientMessage::MergePeer { peer_id, doc_id });
            }
        });
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
