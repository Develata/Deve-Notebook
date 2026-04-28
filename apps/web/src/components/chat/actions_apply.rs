//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use crate::editor::ffi::getEditorContent;
use crate::editor::op_id::next_client_op_id;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate_banner::cannot_action;
use crate::hooks::use_core::{CoreState, pending};
use crate::i18n::{Locale, t};
use deve_core::models::Op;
use deve_core::protocol::{ClientMessage, ServerErrorCode};
use leptos::prelude::*;

pub fn make_on_apply(core: CoreState) -> Callback<String> {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    Callback::new(move |code: String| {
        let Some(doc_id) = core.current_doc.get_untracked() else {
            show_apply_block(&core, "no active document");
            return;
        };
        if !core
            .ws
            .writer_ready_for(core.current_repo_id.get_untracked().as_deref())
        {
            let message = t::server_error::message(
                locale.get_untracked(),
                ServerErrorCode::SyncPeerUnauthenticated,
            );
            let _ = web_sys::window().and_then(|window| window.alert_with_message(message).ok());
            return;
        }
        let utf16_len = getEditorContent().encode_utf16().count();
        let pos = match u32::try_from(utf16_len) {
            Ok(v) => v,
            Err(_) => {
                show_apply_block(&core, "document is too large");
                return;
            }
        };
        let op = Op::Insert {
            pos,
            content: code.into(),
        };
        let Some(client_id) = core
            .ws
            .writer_client_id_for(core.current_repo_id.get_untracked().as_deref())
        else {
            show_apply_block(&core, "writer client id unavailable");
            return;
        };
        let Some(scope_nonce) = stable_local_scope_nonce(LocalScopeSignals {
            current_repo_id: core.current_repo_id,
            current_scope_nonce: core.current_scope_nonce,
            active_branch: core.active_branch,
            pending_branch_switch: core.pending_branch_switch,
            pending_repo_switch: core.pending_repo_switch,
        }) else {
            show_apply_block(&core, "local repo scope is not stable");
            return;
        };
        let client_op_id = next_client_op_id();
        core.set_pending_local_edits.update(|pending_edits| {
            pending::push_pending_edit(
                pending_edits,
                doc_id,
                client_id,
                client_op_id,
                core.doc_version.get_untracked(),
                op.clone(),
            );
        });
        core.ws.send(ClientMessage::Edit {
            doc_id,
            op,
            client_id,
            client_op_id,
            scope_nonce: Some(scope_nonce),
        });
    })
}

fn show_apply_block(core: &CoreState, reason: &str) {
    let message = cannot_action("apply code", reason);
    warn_sync_banner(core.set_sync_banner, message);
}
