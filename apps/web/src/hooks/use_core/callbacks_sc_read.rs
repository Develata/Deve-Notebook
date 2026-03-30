use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::{can_request_doc_diff, to_target};
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use deve_core::protocol::ClientMessage;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::{Callback, Set, WriteSignal};

use super::{SourceControlRequestSignals, SourceControlScopeSignals};

fn log_blocked_sc_read(
    action: &str,
    detail: &str,
    block: crate::hooks::use_core::write_gate::RepoWriteBlock,
) {
    leptos::logging::log!(
        "跳过 {}: source control blocked by {} for {}",
        action,
        block.label(),
        detail
    );
}

pub(super) fn create_get_changes_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    set_request_id: WriteSignal<Option<String>>,
) -> Callback<()> {
    let ws = ws.clone();
    Callback::new(move |_: ()| {
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
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
        if let Some(block) = repo_write_block_untracked(&ws, read_gate) {
            log_blocked_sc_read("GetCommitHistory", &format!("limit={limit}"), block);
            return;
        }
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
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

pub(super) fn create_get_doc_diff_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    set_request_id: WriteSignal<Option<String>>,
) -> Callback<ChangeEntry> {
    let ws = ws.clone();
    Callback::new(move |entry: ChangeEntry| {
        if !can_request_doc_diff(&entry) {
            leptos::logging::log!(
                "跳过 GetDocDiff: deleted change has no doc_id for {}",
                entry.path
            );
            return;
        }
        if let Some(block) = repo_write_block_untracked(&ws, read_gate) {
            log_blocked_sc_read("GetDocDiff", &entry.path, block);
            return;
        }
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_request_id.set(Some(request_id.clone()));
        ws.send(ClientMessage::GetDocDiff {
            request_id,
            target: to_target(&entry),
            scope_nonce: Some(scope_nonce),
        });
    })
}

pub(super) fn create_get_commit_diff_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    set_request_id: WriteSignal<Option<String>>,
) -> Callback<(Option<String>, String)> {
    let ws = ws.clone();
    Callback::new(move |(commit_a, commit_b): (Option<String>, String)| {
        if let Some(block) = repo_write_block_untracked(&ws, read_gate) {
            let detail = match commit_a.as_deref() {
                Some(base) => format!("{base}..{commit_b}"),
                None => commit_b.clone(),
            };
            log_blocked_sc_read("GetCommitDiff", &detail, block);
            return;
        }
        let Some(scope_nonce) = source_control_scope_nonce(scope) else {
            return;
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        set_request_id.set(Some(request_id.clone()));
        ws.send(ClientMessage::GetCommitDiff {
            request_id,
            commit_a,
            commit_b,
            scope_nonce: Some(scope_nonce),
        });
    })
}

pub(super) fn create_read_callbacks(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    request: SourceControlRequestSignals,
) -> (
    Callback<()>,
    Callback<u32>,
    Callback<ChangeEntry>,
    Callback<(Option<String>, String)>,
) {
    (
        create_get_changes_callback(ws, scope, request.set_changes_request_id),
        create_get_history_callback(ws, scope, read_gate, request.set_commit_history_request_id),
        create_get_doc_diff_callback(ws, scope, read_gate, request.set_doc_diff_request_id),
        create_get_commit_diff_callback(ws, scope, read_gate, request.set_commit_diff_request_id),
    )
}
