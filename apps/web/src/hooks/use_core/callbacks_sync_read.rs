use crate::api::WsService;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::super::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};

pub(super) struct SyncReadCallbacks {
    pub(super) on_get_sync_mode: Callback<()>,
    pub(super) on_get_pending_ops: Callback<()>,
    pub(super) on_list_shadows: Callback<()>,
}

pub(super) fn create_sync_read_callbacks(
    ws: &WsService,
    local_scope: LocalScopeSignals,
    set_shadow_list_request_id: WriteSignal<Option<String>>,
    set_sync_mode_request_id: WriteSignal<Option<String>>,
    set_pending_ops_request_id: WriteSignal<Option<String>>,
) -> SyncReadCallbacks {
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
    let on_get_pending_ops = Callback::new(move |_: ()| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 GetPendingOps: local repo scope 尚未稳定");
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_pending_ops_request_id.set(Some(request_id.clone()));
        ws2.send(ClientMessage::GetPendingOps {
            request_id,
            scope_nonce: Some(scope_nonce),
        });
    });

    let ws3 = ws.clone();
    let on_list_shadows = Callback::new(move |_: ()| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            leptos::logging::warn!("忽略 ListShadows: local repo scope 尚未稳定");
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_shadow_list_request_id.set(Some(request_id.clone()));
        ws3.send(ClientMessage::ListShadows {
            request_id,
            scope_nonce: Some(scope_nonce),
        });
    });

    SyncReadCallbacks {
        on_get_sync_mode,
        on_get_pending_ops,
        on_list_shadows,
    }
}
