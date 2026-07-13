use super::{
    clear_stale_doc_diff, create_get_commit_diff_callback, create_get_doc_diff_callback,
    unavailable_doc_diff_notice,
};
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::callbacks_sc::SourceControlScopeSignals;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::RepoWriteSignals;
use crate::hooks::use_core::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::{ClientMessage, ServerErrorCode};
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use leptos::prelude::{Callable, GetUntracked, signal};

#[test]
fn deleted_docless_entry_reports_unavailable_diff_notice() {
    let entry = ChangeEntry {
        path: "deleted.md".into(),
        renamed_from: None,
        doc_id: None,
        status: ChangeStatus::Deleted,
        has_conflict: false,
        domain: Default::default(),
        base_seq: None,
        target_seq: None,
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

#[test]
fn unavailable_doc_diff_clears_stale_session() {
    let (_request_id, set_request_id) = signal(Some("doc-diff-req".to_string()));
    let (notice, set_notice) = signal(None);
    let (diff_content, set_diff_content) = signal(Some(DiffSessionWire::new(
        "note.md".into(),
        "before".into(),
        "after".into(),
    )));

    clear_stale_doc_diff(
        set_request_id,
        set_notice,
        set_diff_content,
        SourceControlNotice {
            code: ServerErrorCode::ScDocNotFound,
            detail: Some("deleted-no-doc-id:deleted.md".into()),
        },
    );

    assert!(diff_content.get_untracked().is_none());
    assert_eq!(
        notice.get_untracked().map(|notice| notice.code),
        Some(ServerErrorCode::ScDocNotFound)
    );
}

#[test]
fn doc_diff_read_gate_allows_remote_branch_spectator_reads() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (current_repo_id, _) = signal(Some("repo-a".to_string()));
    let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
    let (current_scope_nonce, _) = signal(11u64);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    let (load_state, _) = signal(LoadPhase::Ready);
    let (is_spectator, _) = signal(true);
    let (handshake_ready, _) = signal(false);
    let (request_id, set_request_id) = signal(None::<String>);
    let (_notice, set_notice) = signal(None::<SourceControlNotice>);
    let (_diff_content, set_diff_content) = signal(None::<DiffSessionWire>);
    let doc_id = DocId::new();

    let callback = create_get_doc_diff_callback(
        &ws,
        SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        },
        RepoWriteSignals {
            load_state,
            is_spectator: is_spectator.into(),
            handshake_ready,
            current_repo_id,
            current_scope_nonce,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
        },
        set_request_id,
        set_notice,
        set_diff_content,
    );

    callback.run(ChangeEntry {
        path: "notes/a.md".into(),
        renamed_from: None,
        doc_id: Some(doc_id),
        status: ChangeStatus::Modified,
        has_conflict: false,
        domain: Default::default(),
        base_seq: None,
        target_seq: None,
    });

    let request_id = request_id.get_untracked().expect("doc diff request");
    let sent = ws.drain_sent_for_test();
    assert_eq!(sent.len(), 1);
    match &sent[0] {
        ClientMessage::GetDocDiff {
            request_id: sent_request_id,
            target,
            scope_nonce,
        } => {
            assert_eq!(sent_request_id, &request_id);
            assert_eq!(target.doc_id, Some(doc_id));
            assert_eq!(target.path, "notes/a.md");
            assert_eq!(*scope_nonce, Some(11));
        }
        other => panic!("expected GetDocDiff, got {other:?}"),
    }
}

#[test]
fn commit_diff_read_gate_allows_remote_branch_spectator_reads() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (current_repo_id, _) = signal(Some("repo-a".to_string()));
    let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
    let (current_scope_nonce, _) = signal(13u64);
    let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);
    let (load_state, _) = signal(LoadPhase::Ready);
    let (is_spectator, _) = signal(true);
    let (handshake_ready, _) = signal(false);
    let (request_id, set_request_id) = signal(None::<String>);

    let callback = create_get_commit_diff_callback(
        &ws,
        SourceControlScopeSignals {
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
        },
        RepoWriteSignals {
            load_state,
            is_spectator: is_spectator.into(),
            handshake_ready,
            current_repo_id,
            current_scope_nonce,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
        },
        set_request_id,
    );

    callback.run((Some("base".to_string()), "head".to_string()));

    let request_id = request_id.get_untracked().expect("commit diff request");
    let sent = ws.drain_sent_for_test();
    assert_eq!(sent.len(), 1);
    match &sent[0] {
        ClientMessage::GetCommitDiff {
            request_id: sent_request_id,
            commit_a,
            commit_b,
            scope_nonce,
        } => {
            assert_eq!(sent_request_id, &request_id);
            assert_eq!(commit_a.as_deref(), Some("base"));
            assert_eq!(commit_b, "head");
            assert_eq!(*scope_nonce, Some(13));
        }
        other => panic!("expected GetCommitDiff, got {other:?}"),
    }
}
