//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 15_release#runtime-observability
//!
use crate::api::WsService;
use leptos::prelude::{Get, Signal};

use super::state::CoreSignals;
use super::status_summary::derive_sync_status;

pub(super) fn build_status_text(ws: &WsService, signals: &CoreSignals) -> Signal<String> {
    let status_signal_for_text = ws.status;
    let degraded_for_text = signals.degraded_sync_mode;
    let load_state_for_text = signals.load_state;
    let active_branch_for_text = signals.active_branch;
    let handshake_ready_for_text = signals.handshake_ready;
    let current_repo_id_for_text = signals.current_repo_id;
    let current_repo_for_text = signals.current_repo;
    let pending_repo_switch_for_text = signals.pending_repo_switch;
    let pending_branch_switch_for_text = signals.pending_branch_switch;
    let current_doc_for_text = signals.current_doc;
    let pending_edits_for_text = signals.pending_local_edits;
    let ws_for_text = ws.clone();
    Signal::derive(move || {
        let current_doc = current_doc_for_text.get();
        let pending_ack_count = current_doc
            .and_then(|doc_id| pending_edits_for_text.get().get(&doc_id).map(Vec::len))
            .unwrap_or_default();
        derive_sync_status(
            status_signal_for_text.get(),
            &load_state_for_text.get(),
            active_branch_for_text.get().is_some(),
            degraded_for_text.get().is_some(),
            handshake_ready_for_text.get(),
            ws_for_text.writer_ready_for(current_repo_id_for_text.get().as_deref()),
            current_repo_id_for_text.get().as_deref(),
            current_repo_for_text.get().as_deref(),
            pending_repo_switch_for_text.get().as_deref(),
            pending_branch_switch_for_text.get().is_some(),
            pending_ack_count,
        )
        .header_text()
        .to_string()
    })
}
