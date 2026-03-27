use super::EditorStats;
use super::ffi::{Delta, getEditorContent};
use super::op_id::next_client_op_id;
use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::pending::{self, PendingLocalEdits};
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;
use wasm_bindgen::prelude::Closure;

pub struct DeltaInputCtx {
    pub doc_id: DocId,
    pub ws: WsService,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub handshake_ready: ReadSignal<bool>,
    pub is_playback: ReadSignal<bool>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub local_version: ReadSignal<u64>,
    pub on_stats: Option<Callback<EditorStats>>,
    pub set_content: WriteSignal<String>,
}

pub fn build_on_delta(ctx: DeltaInputCtx) -> Closure<dyn FnMut(String)> {
    Closure::wrap(Box::new(move |delta_json: String| {
        let deltas: Vec<Delta> = match serde_json::from_str(&delta_json) {
            Ok(deltas) => deltas,
            Err(err) => {
                leptos::logging::error!("Delta 解析失败: {:?}", err);
                return;
            }
        };
        let writer_ready = ctx
            .ws
            .writer_ready_for(ctx.current_repo_id.get_untracked().as_deref());
        let can_forward = can_send_delta(
            ctx.is_playback.get_untracked(),
            ctx.pending_branch_switch.get_untracked().is_some(),
            ctx.pending_repo_switch.get_untracked().is_some(),
            ctx.handshake_ready.get_untracked(),
            writer_ready,
        );

        if can_forward && !deltas.is_empty() {
            let Some(client_id) = ctx
                .ws
                .writer_client_id_for(ctx.current_repo_id.get_untracked().as_deref())
            else {
                leptos::logging::warn!("Delta ignored: writer client id unavailable.");
                sync_local_state(ctx.on_stats, ctx.set_content);
                return;
            };
            let Some(scope_nonce) = stable_local_scope_nonce(LocalScopeSignals {
                current_repo_id: ctx.current_repo_id,
                current_scope_nonce: ctx.current_scope_nonce,
                active_branch: ctx.active_branch,
                pending_branch_switch: ctx.pending_branch_switch,
                pending_repo_switch: ctx.pending_repo_switch,
            }) else {
                leptos::logging::warn!("Delta ignored: local scope nonce unavailable.");
                sync_local_state(ctx.on_stats, ctx.set_content);
                return;
            };
            for delta in deltas {
                for op in delta.to_ops() {
                    let client_op_id = next_client_op_id();
                    ctx.set_pending_local_edits.update(|pending_edits| {
                        pending::push_pending_edit(
                            pending_edits,
                            ctx.doc_id,
                            client_id,
                            client_op_id,
                            ctx.local_version.get_untracked(),
                            op.clone(),
                        );
                    });
                    ctx.ws.send(ClientMessage::Edit {
                        doc_id: ctx.doc_id,
                        op: op.clone(),
                        client_id,
                        client_op_id,
                        scope_nonce: Some(scope_nonce),
                    });
                }
            }
        }
        sync_local_state(ctx.on_stats, ctx.set_content);
    }) as Box<dyn FnMut(String)>)
}

fn sync_local_state(on_stats: Option<Callback<EditorStats>>, set_content: WriteSignal<String>) {
    let text = getEditorContent();
    emit_stats(on_stats, &text);
    set_content.set(text);
}

fn emit_stats(on_stats: Option<Callback<EditorStats>>, text: &str) {
    if let Some(cb) = on_stats {
        cb.run(EditorStats {
            chars: text.len(),
            words: text.split_whitespace().count(),
            lines: text.lines().count(),
        });
    }
}

fn can_send_delta(
    is_playback: bool,
    branch_switch_pending: bool,
    repo_switch_pending: bool,
    handshake_ready: bool,
    writer_ready: bool,
) -> bool {
    !is_playback
        && !branch_switch_pending
        && !repo_switch_pending
        && handshake_ready
        && writer_ready
}

#[cfg(test)]
mod tests {
    use super::can_send_delta;

    #[test]
    fn blocks_delta_while_scope_switch_is_pending() {
        assert!(!can_send_delta(false, true, false, true, true));
        assert!(!can_send_delta(false, false, true, true, true));
    }

    #[test]
    fn blocks_delta_before_handshake_is_ready() {
        assert!(!can_send_delta(false, false, false, false, true));
    }

    #[test]
    fn allows_delta_only_in_stable_writable_scope() {
        assert!(can_send_delta(false, false, false, true, true));
        assert!(!can_send_delta(false, false, false, true, false));
    }
}
