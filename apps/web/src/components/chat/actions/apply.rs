//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::editor::ffi::{applyRemoteOp, getEditorContent, sync_editor_state_to_rust};
use crate::editor::op_id::next_client_op_id;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::contexts::EditorContext;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::cannot_action;
use crate::runtime::document::pending;
use crate::runtime::session_client::SessionClient;
use deve_core::models::{Op, RepoId};
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ApplyEditPlanError {
    DocumentTooLarge,
}

#[derive(Clone)]
pub struct ChatApplyRuntime {
    pub session: SessionClient,
    pub editor: EditorContext,
}

pub fn make_on_apply(runtime: ChatApplyRuntime) -> Callback<String> {
    Callback::new(move |code: String| {
        let Some(doc_id) = runtime.editor.current_doc.get_untracked() else {
            show_apply_block(&runtime.session, "no active document");
            return;
        };
        if let Some(block) = repo_write_block_untracked(
            &runtime.session.ws,
            RepoWriteSignals {
                load_state: runtime.editor.load_state,
                is_spectator: runtime.editor.is_spectator,
                handshake_ready: runtime.editor.handshake_ready,
                current_repo_id: runtime.editor.current_repo_id,
                current_scope_nonce: runtime.editor.current_scope_nonce,
                active_branch: runtime.editor.active_branch,
                pending_branch_switch: runtime.editor.pending_branch_switch,
                pending_repo_switch: runtime.editor.pending_repo_switch,
            },
        ) {
            show_apply_block(&runtime.session, block.label());
            return;
        }
        let op = match build_append_markdown_op(&getEditorContent(), code) {
            Ok(op) => op,
            Err(ApplyEditPlanError::DocumentTooLarge) => {
                show_apply_block(&runtime.session, "document is too large");
                return;
            }
        };
        let Some(scope_nonce) = stable_local_scope_nonce(LocalScopeSignals {
            current_repo_id: runtime.editor.current_repo_id,
            current_scope_nonce: runtime.editor.current_scope_nonce,
            active_branch: runtime.editor.active_branch,
            pending_branch_switch: runtime.editor.pending_branch_switch,
            pending_repo_switch: runtime.editor.pending_repo_switch,
        }) else {
            show_apply_block(&runtime.session, "local repo scope is not stable");
            return;
        };
        let Some(client_id) = runtime.session.ws.writer_client_id_for(
            runtime.editor.current_repo_id.get_untracked().as_deref(),
            Some(scope_nonce),
        ) else {
            show_apply_block(&runtime.session, "writer client id unavailable");
            return;
        };
        let Some(repo_id) = runtime
            .editor
            .current_repo_id
            .get_untracked()
            .and_then(|repo_id| repo_id.parse::<RepoId>().ok())
        else {
            show_apply_block(&runtime.session, "current repo id unavailable");
            return;
        };
        if !apply_local_programmatic_op(&op) {
            show_apply_block(&runtime.session, "failed to apply code locally");
            return;
        }
        let client_op_id = next_client_op_id();
        runtime
            .editor
            .set_pending_local_edits
            .update(|pending_edits| {
                pending::push_pending_edit(
                    pending_edits,
                    pending::PendingLocalEditInput {
                        repo_id,
                        doc_id,
                        scope_nonce,
                        client_id,
                        client_op_id,
                        base_version: runtime.editor.doc_version.get_untracked(),
                        op: op.clone(),
                    },
                );
            });
        runtime.session.ws.send(build_apply_edit_message(
            doc_id,
            op,
            client_id,
            client_op_id,
            scope_nonce,
        ));
    })
}

fn apply_local_programmatic_op(op: &Op) -> bool {
    let Ok(json) = serde_json::to_string(op) else {
        return false;
    };
    applyRemoteOp(&json);
    sync_editor_state_to_rust();
    true
}

fn show_apply_block(session: &SessionClient, reason: &str) {
    let message = cannot_action("apply code", reason);
    warn_sync_banner(session.set_sync_banner, message);
}

fn build_append_markdown_op(current_content: &str, code: String) -> Result<Op, ApplyEditPlanError> {
    let utf16_len = current_content.encode_utf16().count();
    build_append_markdown_op_at_utf16_len(utf16_len, code)
}

fn build_append_markdown_op_at_utf16_len(
    utf16_len: usize,
    code: String,
) -> Result<Op, ApplyEditPlanError> {
    let pos = u32::try_from(utf16_len).map_err(|_| ApplyEditPlanError::DocumentTooLarge)?;
    Ok(Op::Insert {
        pos,
        content: code.into(),
    })
}

fn build_apply_edit_message(
    doc_id: deve_core::models::DocId,
    op: Op,
    client_id: u64,
    client_op_id: u64,
    scope_nonce: u64,
) -> ClientMessage {
    ClientMessage::Edit {
        doc_id,
        op,
        client_id,
        client_op_id,
        scope_nonce: Some(scope_nonce),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplyEditPlanError, build_append_markdown_op, build_append_markdown_op_at_utf16_len,
        build_apply_edit_message,
    };
    use deve_core::models::{DocId, Op};
    use deve_core::protocol::ClientMessage;

    #[test]
    fn chat_apply_append_markdown_op_uses_utf16_end_position() {
        assert_eq!(
            build_append_markdown_op("a🙂", " patch".to_string()),
            Ok(Op::Insert {
                pos: 3,
                content: " patch".into(),
            })
        );
    }

    #[test]
    fn chat_apply_append_markdown_op_fails_closed_when_position_overflows() {
        assert_eq!(
            build_append_markdown_op_at_utf16_len(u32::MAX as usize + 1, " patch".to_string()),
            Err(ApplyEditPlanError::DocumentTooLarge)
        );
    }

    #[test]
    fn chat_apply_edit_message_carries_current_scope_nonce() {
        let doc_id = DocId::from_u128(7);
        let op = Op::Insert {
            pos: 3,
            content: " patch".into(),
        };

        match build_apply_edit_message(doc_id, op.clone(), 11, 13, 17) {
            ClientMessage::Edit {
                doc_id: actual_doc_id,
                op: actual_op,
                client_id,
                client_op_id,
                scope_nonce,
            } => {
                assert_eq!(actual_doc_id, doc_id);
                assert_eq!(actual_op, op);
                assert_eq!(client_id, 11);
                assert_eq!(client_op_id, 13);
                assert_eq!(scope_nonce, Some(17));
            }
            other => panic!("expected Edit message, got {other:?}"),
        }
    }
}
