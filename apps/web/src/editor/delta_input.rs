use super::EditorStats;
use super::ffi::{Delta, getEditorContent};
use super::op_id::next_client_op_id;
use crate::api::WsService;
use crate::hooks::use_core::pending::{self, PendingLocalEdits};
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;
use wasm_bindgen::prelude::Closure;

pub struct DeltaInputCtx {
    pub doc_id: DocId,
    pub ws: WsService,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub is_playback: ReadSignal<bool>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub local_version: ReadSignal<u64>,
    pub on_stats: Option<Callback<EditorStats>>,
    pub set_content: WriteSignal<String>,
}

pub fn build_on_delta(ctx: DeltaInputCtx) -> Closure<dyn FnMut(String)> {
    Closure::wrap(Box::new(move |delta_json: String| {
        if ctx.is_playback.get_untracked() {
            return;
        }
        let Some(client_id) = ctx
            .ws
            .writer_client_id_for(ctx.current_repo_id.get_untracked().as_deref())
        else {
            leptos::logging::warn!("Delta ignored: writer client id unavailable.");
            return;
        };
        let deltas: Vec<Delta> = match serde_json::from_str(&delta_json) {
            Ok(deltas) => deltas,
            Err(err) => {
                leptos::logging::error!("Delta 解析失败: {:?}", err);
                return;
            }
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
                });
            }
        }
        let text = getEditorContent();
        emit_stats(ctx.on_stats, &text);
        ctx.set_content.set(text);
    }) as Box<dyn FnMut(String)>)
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
