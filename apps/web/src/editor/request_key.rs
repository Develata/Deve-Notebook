//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::EditorContext;
use deve_core::protocol::ClientMessage;
use deve_core::security::RepoKey;
use leptos::prelude::*;

pub fn setup_request_key_effect(
    ws: WsService,
    core: EditorContext,
    set_repo_key: WriteSignal<Option<RepoKey>>,
) {
    Effect::new(move |_| {
        if core.active_branch.get().is_some() || !core.handshake_ready.get() {
            set_repo_key.set(None);
            return;
        }
        if ws.status.get() == ConnectionStatus::Connected {
            ws.send(ClientMessage::RequestKey {
                scope_nonce: Some(core.current_scope_nonce.get()),
            });
        }
    });
}
