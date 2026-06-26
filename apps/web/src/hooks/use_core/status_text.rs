//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 18_release#runtime-observability
//!
use crate::api::WsService;
use leptos::prelude::{Get, Signal};

use super::state::CoreSignals;
use super::status_summary::{SyncStatusInput, derive_sync_status};
use crate::runtime::document::pending::{PendingScope, pending_count_for_doc_in_scope};

pub(super) fn build_status_text(ws: &WsService, signals: &CoreSignals) -> Signal<String> {
    let status_signal_for_text = ws.status;
    let degraded_for_text = signals.degraded_sync_mode;
    let load_state_for_text = signals.load_state;
    let active_branch_for_text = signals.active_branch;
    let handshake_ready_for_text = signals.handshake_ready;
    let current_repo_id_for_text = signals.current_repo_id;
    let current_scope_nonce_for_text = signals.current_scope_nonce;
    let current_repo_for_text = signals.current_repo;
    let pending_repo_switch_for_text = signals.pending_repo_switch;
    let pending_branch_switch_for_text = signals.pending_branch_switch;
    let current_doc_for_text = signals.current_doc;
    let pending_edits_for_text = signals.pending_local_edits;
    let ws_for_text = ws.clone();
    Signal::derive(move || {
        let current_repo_id = current_repo_id_for_text.get();
        let current_scope_nonce = current_scope_nonce_for_text.get();
        let handshake_ready = handshake_ready_for_text.get();
        let readiness = ws_for_text.native_runtime_readiness_for(
            current_repo_id.as_deref(),
            Some(current_scope_nonce),
            handshake_ready,
        );
        let current_doc = current_doc_for_text.get();
        let pending_ack_count = current_doc
            .and_then(|doc_id| {
                PendingScope::from_repo_id_str(current_repo_id.as_deref(), current_scope_nonce).map(
                    |scope| {
                        pending_count_for_doc_in_scope(&pending_edits_for_text.get(), doc_id, scope)
                    },
                )
            })
            .unwrap_or_default();
        derive_sync_status(SyncStatusInput {
            connection_status: status_signal_for_text.get(),
            load_state: load_state_for_text.get().as_str(),
            remote_branch_active: active_branch_for_text.get().is_some(),
            degraded_storage: degraded_for_text.get().is_some(),
            node_role_probe_failed: ws_for_text.node_role_probe_failed.get(),
            node_role_readable: readiness.node_role_readable,
            handshake_ready: readiness.repo_handshake_complete,
            writer_ready: readiness.writer_ready,
            current_repo_id: current_repo_id.as_deref(),
            current_repo_name: current_repo_for_text.get().as_deref(),
            pending_repo_switch: pending_repo_switch_for_text.get().as_deref(),
            pending_branch_switch: pending_branch_switch_for_text.get().is_some(),
            pending_ack_count,
        })
        .header_text()
        .to_string()
    })
}
