//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use super::EditorStats;
use super::delta::Delta;
use super::delta_input_forward::forward_deltas;
use super::delta_input_gate::can_send_delta;
use super::delta_input_state::sync_local_state;
use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::runtime::document::pending::PendingLocalEdits;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use deve_core::models::{DocId, PeerId};
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
    pub load_state: ReadSignal<String>,
    pub is_spectator: Signal<bool>,
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
        let write_allowed = repo_write_block_untracked(
            &ctx.ws,
            RepoWriteSignals {
                load_state: ctx.load_state,
                is_spectator: ctx.is_spectator,
                handshake_ready: ctx.handshake_ready,
                current_repo_id: ctx.current_repo_id,
                current_scope_nonce: ctx.current_scope_nonce,
                active_branch: ctx.active_branch,
                pending_branch_switch: ctx.pending_branch_switch,
                pending_repo_switch: ctx.pending_repo_switch,
            },
        )
        .is_none();
        let can_forward = can_send_delta(ctx.is_playback.get_untracked(), write_allowed);

        if can_forward && !deltas.is_empty() && !forward_deltas(&ctx, deltas) {
            sync_local_state(ctx.on_stats, ctx.set_content);
            return;
        }
        sync_local_state(ctx.on_stats, ctx.set_content);
    }) as Box<dyn FnMut(String)>)
}
