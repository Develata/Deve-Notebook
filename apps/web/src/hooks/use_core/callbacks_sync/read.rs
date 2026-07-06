//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason, cannot_send};
use crate::i18n::Locale;
use crate::runtime::scope_client::{LocalScopeSignals, stable_local_scope_nonce};
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

pub(super) struct SyncReadCallbacks {
    pub(super) on_get_sync_mode: Callback<()>,
    pub(super) on_get_pending_ops: Callback<()>,
    pub(super) on_list_shadows: Callback<()>,
}

pub(super) fn create_sync_read_callbacks(
    ws: &WsService,
    locale: RwSignal<Locale>,
    local_scope: LocalScopeSignals,
    set_shadow_list_request_id: WriteSignal<Option<String>>,
    set_sync_mode_request_id: WriteSignal<Option<String>>,
    set_pending_ops_request_id: WriteSignal<Option<String>>,
    set_sync_banner: WriteSignal<Option<String>>,
) -> SyncReadCallbacks {
    let ws1 = ws.clone();
    let on_get_sync_mode = Callback::new(move |_: ()| {
        let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
            show_sync_read_block(set_sync_banner, locale, WriteGateAction::GetSyncMode);
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
            show_sync_read_block(set_sync_banner, locale, WriteGateAction::GetPendingOps);
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
            show_sync_read_block(set_sync_banner, locale, WriteGateAction::ListShadows);
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

fn show_sync_read_block(
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    action: WriteGateAction,
) {
    let message = cannot_send(
        locale.get_untracked(),
        action,
        WriteGateReason::LocalRepoScopeUnstable,
    );
    warn_sync_banner(set_sync_banner, message);
}

#[cfg(test)]
mod tests {
    use super::show_sync_read_block;
    use crate::hooks::use_core::write_gate_banner::WriteGateAction;
    use crate::i18n::Locale;
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn sync_read_block_banner_includes_action_and_scope_reason() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);
        let locale = leptos::prelude::RwSignal::new(Locale::Zh);

        show_sync_read_block(set_sync_banner, locale, WriteGateAction::ListShadows);

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("无法发送 列出远端影子请求：本地仓库作用域不稳定")
        );
    }
}
