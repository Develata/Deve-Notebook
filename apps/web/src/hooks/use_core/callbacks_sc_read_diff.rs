use crate::api::WsService;
use crate::hooks::use_core::callbacks_sc_scope::source_control_scope_nonce;
use crate::hooks::use_core::callbacks_sc_target::{can_request_doc_diff, to_target};
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::{
    RepoWriteSignals, repo_source_control_read_block_untracked, repo_write_block_untracked,
};
use deve_core::protocol::{ClientMessage, ServerErrorCode};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::{Callback, Set, WriteSignal};

use super::{SourceControlScopeSignals, log_blocked_sc_read};

const DELETED_NO_DOC_ID_NOTICE_PREFIX: &str = "deleted-no-doc-id:";

fn unavailable_doc_diff_notice(entry: &ChangeEntry) -> Option<SourceControlNotice> {
    (!can_request_doc_diff(entry)).then(|| SourceControlNotice {
        code: ServerErrorCode::ScDocNotFound,
        detail: Some(format!("{DELETED_NO_DOC_ID_NOTICE_PREFIX}{}", entry.path)),
    })
}

pub(super) fn create_get_doc_diff_callback(
    ws: &WsService,
    scope: SourceControlScopeSignals,
    read_gate: RepoWriteSignals,
    set_request_id: WriteSignal<Option<String>>,
    set_notice: WriteSignal<Option<SourceControlNotice>>,
) -> Callback<ChangeEntry> {
    let ws = ws.clone();
    Callback::new(move |entry: ChangeEntry| {
        if let Some(notice) = unavailable_doc_diff_notice(&entry) {
            set_notice.set(Some(notice));
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
        if let Some(block) = repo_source_control_read_block_untracked(&ws, read_gate) {
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

#[cfg(test)]
mod tests {
    use super::unavailable_doc_diff_notice;
    use deve_core::protocol::ServerErrorCode;
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

    #[test]
    fn deleted_docless_entry_reports_unavailable_diff_notice() {
        let entry = ChangeEntry {
            path: "deleted.md".into(),
            renamed_from: None,
            doc_id: None,
            status: ChangeStatus::Deleted,
            has_conflict: false,
        };
        let notice = unavailable_doc_diff_notice(&entry).expect("notice");
        assert_eq!(notice.code, ServerErrorCode::ScDocNotFound);
        assert!(
            notice
                .detail
                .as_deref()
                .is_some_and(|detail| detail.ends_with("deleted.md"))
        );
    }
}
