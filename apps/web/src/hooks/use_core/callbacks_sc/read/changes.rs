//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_read_scope_nonce;
use crate::hooks::use_core::write_gate::{
    RepoWriteSignals, repo_source_control_read_block_untracked,
};
use deve_core::protocol::ClientMessage;
use leptos::prelude::{Callback, Set, WriteSignal};

use super::{SourceControlScopeSignals, log_blocked_sc_read};

pub(super) fn create_get_changes_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    set_request_id: WriteSignal<Option<String>>,
) -> Callback<()> {
    let ws = ws.clone();
    Callback::new(move |_: ()| {
        if let Some(block) = repo_source_control_read_block_untracked(&ws, read_gate) {
            log_blocked_sc_read("GetChanges", "working tree", block);
            return;
        }
        let Some(scope_nonce) = source_control_read_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_request_id.set(Some(request_id.clone()));
        ws.send(ClientMessage::GetChanges {
            request_id,
            scope_nonce: Some(scope_nonce),
        });
    })
}

pub(super) fn create_get_history_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    set_request_id: WriteSignal<Option<String>>,
) -> Callback<u32> {
    let ws = ws.clone();
    Callback::new(move |limit: u32| {
        if let Some(block) = repo_source_control_read_block_untracked(&ws, read_gate) {
            log_blocked_sc_read("GetCommitHistory", &format!("limit={limit}"), block);
            return;
        }
        let Some(scope_nonce) = source_control_read_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_request_id.set(Some(request_id.clone()));
        ws.send(ClientMessage::GetCommitHistory {
            request_id,
            limit,
            scope_nonce: Some(scope_nonce),
        });
    })
}
