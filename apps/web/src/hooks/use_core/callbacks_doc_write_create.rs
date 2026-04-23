use crate::api::WsService;
use crate::hooks::use_core::callbacks_scope::LocalScopeSignals;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::scope::local_write_scope_nonce;

pub(super) fn create_doc_create_callback(
    ws: &WsService,
    current_doc: ReadSignal<Option<deve_core::models::DocId>>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    set_pending_created_doc_path: WriteSignal<Option<String>>,
    set_explicit_home: WriteSignal<bool>,
) -> Callback<String> {
    let ws = ws.clone();
    Callback::new(move |name: String| {
        let Some(scope_nonce) =
            local_write_scope_nonce(&ws, local_scope, write_gate, set_sync_banner, "CreateDoc")
        else {
            return;
        };
        if current_doc.get_untracked().is_none() {
            set_explicit_home.set(false);
            set_pending_created_doc_path.set(Some(name.clone()));
        } else {
            set_pending_created_doc_path.set(None);
        }
        ws.send(ClientMessage::CreateDoc {
            name,
            scope_nonce: Some(scope_nonce),
        });
    })
}
