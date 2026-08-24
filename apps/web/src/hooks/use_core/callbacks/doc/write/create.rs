//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 09_web_thin_client_ledger#document-create-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::write_gate_banner::WriteGateAction;
use crate::runtime::document::create::PendingDocumentCreate;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

use super::DocWriteSignals;
use super::scope::local_write_scope_nonce;

pub(super) fn create_doc_create_callback(
    ws: &WsService,
    signals: DocWriteSignals,
) -> Callback<String> {
    let ws = ws.clone();
    Callback::new(move |name: String| {
        let Some(scope_nonce) = local_write_scope_nonce(
            &ws,
            signals.locale,
            signals.local_scope,
            signals.write_gate,
            signals.set_sync_banner,
            WriteGateAction::CreateDoc,
        ) else {
            return;
        };
        if signals.pending_document_create.get_untracked().is_some() {
            leptos::logging::warn!("Document Create ignored while one typed intent is pending");
            return;
        }
        let Some(repo_id) = signals
            .local_scope
            .current_repo_id
            .get_untracked()
            .and_then(|repo_id| repo_id.parse().ok())
        else {
            return;
        };
        let select_when_projected = signals.current_doc.get_untracked().is_none();
        if select_when_projected {
            signals.set_explicit_home.set(false);
        }
        let pending = PendingDocumentCreate::new(repo_id, scope_nonce, name, select_when_projected);
        let request = pending.request();
        signals.set_pending_document_create.set(Some(pending));
        ws.send(ClientMessage::DocumentCreate(request));
    })
}
