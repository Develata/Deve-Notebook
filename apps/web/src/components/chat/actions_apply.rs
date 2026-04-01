use crate::editor::ffi::getEditorContent;
use crate::editor::op_id::next_client_op_id;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::{CoreState, pending};
use crate::i18n::{Locale, t};
use deve_core::models::Op;
use deve_core::protocol::{ClientMessage, ServerErrorCode};
use leptos::prelude::*;

pub fn make_on_apply(core: CoreState) -> Callback<String> {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    Callback::new(move |code: String| {
        let Some(doc_id) = core.current_doc.get_untracked() else {
            leptos::logging::warn!("No active doc to apply code.");
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
                leptos::logging::warn!("Apply code aborted: UTF-16 length overflow.");
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
            leptos::logging::warn!("Apply code aborted: writer client id unavailable.");
            return;
        };
        let Some(scope_nonce) = stable_local_scope_nonce(LocalScopeSignals {
            current_repo_id: core.current_repo_id,
            current_scope_nonce: core.current_scope_nonce,
            active_branch: core.active_branch,
            pending_branch_switch: core.pending_branch_switch,
            pending_repo_switch: core.pending_repo_switch,
        }) else {
            leptos::logging::warn!("Apply code aborted: local scope nonce unavailable.");
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
