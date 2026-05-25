//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_read_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::{can_request_doc_diff, to_target};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::source_control_notice::{
    DELETED_NO_DOC_ID_NOTICE_PREFIX, SourceControlNotice,
};
use crate::hooks::use_core::write_gate::{
    RepoWriteSignals, repo_source_control_read_block_untracked,
};
use deve_core::protocol::{ClientMessage, ServerErrorCode};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::{Callback, Set, WriteSignal};

use super::{SourceControlScopeSignals, log_blocked_sc_read};

fn unavailable_doc_diff_notice(entry: &ChangeEntry) -> Option<SourceControlNotice> {
    (!can_request_doc_diff(entry)).then(|| SourceControlNotice {
        code: ServerErrorCode::ScDocNotFound,
        detail: Some(format!("{DELETED_NO_DOC_ID_NOTICE_PREFIX}{}", entry.path)),
    })
}

fn clear_stale_doc_diff(
    set_request_id: WriteSignal<Option<String>>,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
    set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    notice: SourceControlNotice,
) {
    set_request_id.set(None);
    set_diff_content.set(None);
    set_notice.set(Some(notice));
}

pub(super) fn create_get_doc_diff_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    set_request_id: WriteSignal<Option<String>>,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
    set_diff_content: WriteSignal<Option<DiffSessionWire>>,
) -> Callback<ChangeEntry> {
    let ws = ws.clone();
    Callback::new(move |entry: ChangeEntry| {
        if let Some(notice) = unavailable_doc_diff_notice(&entry) {
            clear_stale_doc_diff(set_request_id, set_notice, set_diff_content, notice);
            return;
        }
        if let Some(block) = repo_source_control_read_block_untracked(&ws, read_gate) {
            log_blocked_sc_read("GetDocDiff", &entry.path, block);
            return;
        }
        let Some(scope_nonce) = source_control_read_scope_nonce(scope) else {
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
        if let Some(block) = repo_source_control_read_block_untracked(&ws, read_gate) {
            let detail = match commit_a.as_deref() {
                Some(base) => format!("{base}..{commit_b}"),
                None => commit_b.clone(),
            };
            log_blocked_sc_read("GetCommitDiff", &detail, block);
            return;
        }
        let Some(scope_nonce) = source_control_read_scope_nonce(scope) else {
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

#[cfg(test)]
mod tests;
