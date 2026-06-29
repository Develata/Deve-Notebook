//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::write_gate_banner::WriteGateAction;
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
        if signals.current_doc.get_untracked().is_none() {
            signals.set_explicit_home.set(false);
            signals.set_pending_created_doc_path.set(Some(name.clone()));
        } else {
            signals.set_pending_created_doc_path.set(None);
        }
        ws.send(ClientMessage::CreateDoc {
            name,
            scope_nonce: Some(scope_nonce),
        });
    })
}
