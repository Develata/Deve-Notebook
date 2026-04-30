//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 12_commands#cli-commands

use crate::server::{
    channel::DualChannel,
    handlers::source_control::handle_resolve_conflict,
    session::WsSession,
};
use deve_core::git_bridge::apply_import;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage};
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, ConflictResolution};
use tokio::sync::mpsc;

#[allow(dead_code)]
#[path = "source_control_git_import_test_support.rs"]
mod support;

use support::{
    build_state, create_imported_conflict_fixture, git, init_git_repo, write_workspace_file,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_conflict_keep_fs_resolves_to_clean_staged_entry() -> anyhow::Result<()> {
    let fixture = create_imported_conflict_fixture()?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(fixture.state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(fixture.repo_name.clone(), None);
    session.set_scope_nonce(Some(31));

    handle_resolve_conflict(
        &fixture.state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(fixture.doc_id),
        },
        ConflictResolution::KeepFs,
    )
    .await;

    match uni_rx.recv().await.expect("conflict resolved") {
        ServerMessage::ConflictResolved {
            repo_id: actual_repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
            ..
        } => {
            assert_eq!(actual_repo_id, Some(fixture.repo_id));
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(31));
            assert_eq!(path, "note.md");
            assert_eq!(resolution, "KeepFs");
        }
        other => panic!("expected ConflictResolved, got {other:?}"),
    }
    let pending = fixture
        .state
        .repo
        .list_pending_fs_in_local_repo(&fixture.repo_name)?;
    assert!(pending.is_empty(), "{pending:?}");
    let staged = fixture
        .state
        .repo
        .list_staged_in_local_repo(&fixture.repo_name)?;
    assert_eq!(staged.len(), 1, "{staged:?}");
    assert_eq!(staged[0].path, "note.md");
    assert!(!staged[0].has_conflict, "{staged:?}");
    let after_commits = fixture
        .state
        .repo
        .list_commits_in_local_repo(&fixture.repo_name, 10)?;
    assert_eq!(after_commits.len(), fixture.before_commit_count);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_conflict_keep_ledger_discards_import_without_staging() -> anyhow::Result<()> {
    let fixture = create_imported_conflict_fixture()?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(fixture.state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(fixture.repo_name.clone(), None);
    session.set_scope_nonce(Some(32));

    handle_resolve_conflict(
        &fixture.state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(fixture.doc_id),
        },
        ConflictResolution::KeepLedger,
    )
    .await;

    match uni_rx.recv().await.expect("conflict resolved") {
        ServerMessage::ConflictResolved {
            repo_id: actual_repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
            ..
        } => {
            assert_eq!(actual_repo_id, Some(fixture.repo_id));
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(32));
            assert_eq!(path, "note.md");
            assert_eq!(resolution, "KeepLedger");
        }
        other => panic!("expected ConflictResolved, got {other:?}"),
    }
    let pending = fixture
        .state
        .repo
        .list_pending_fs_in_local_repo(&fixture.repo_name)?;
    assert!(pending.is_empty(), "{pending:?}");
    let staged = fixture
        .state
        .repo
        .list_staged_in_local_repo(&fixture.repo_name)?;
    assert!(staged.is_empty(), "{staged:?}");
    let restored = std::fs::read_to_string(fixture.repo_root.join("note.md"))?;
    assert_eq!(restored, "hello\nledger\n");
    let after_commits = fixture
        .state
        .repo
        .list_commits_in_local_repo(&fixture.repo_name, 10)?;
    assert_eq!(after_commits.len(), fixture.before_commit_count);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_conflict_rejects_non_conflict_pending_entry() -> anyhow::Result<()> {
    let (dir, state, _repo_id, _test_id) = build_state()?;
    let repo_name = state.repo.local_repo_name().to_string();
    write_workspace_file(&dir, &repo_name, "note.md", "plain pending\n");
    state.repo.run_on_local_repo(&repo_name, |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "note.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("plain pending\n"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(repo_name.clone(), None);
    session.set_scope_nonce(Some(35));

    handle_resolve_conflict(
        &state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "note.md".into(),
            doc_id: None,
        },
        ConflictResolution::KeepFs,
    )
    .await;

    match uni_rx.recv().await.expect("protocol error") {
        ServerMessage::ProtocolError {
            error, scope_nonce, ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScConflictTargetMissing);
            assert_eq!(error.detail.as_deref(), Some("Source control target is not a conflict: note.md"));
            assert_eq!(scope_nonce, Some(35));
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
    let pending = state.repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0].path, "note.md");
    let staged = state.repo.list_staged_in_local_repo(&repo_name)?;
    assert!(staged.is_empty(), "{staged:?}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_rename_conflict_keep_fs_stages_single_clean_entry() -> anyhow::Result<()> {
    let (dir, state, repo_id, _test_id) = build_state()?;
    let repo_name = state.repo.local_repo_name().to_string();
    let repo_root = state.repo.local_repo_workspace_root(&repo_name)?;
    init_git_repo(&repo_root);
    let baseline = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\n";
    write_workspace_file(&dir, &repo_name, "note.md", baseline);
    state.repo.run_on_local_repo(&repo_name, |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "note.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash(baseline),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;
    state
        .repo
        .stage_pending_in_local_repo(&repo_name, "note.md")?;
    state
        .repo
        .commit_staged_in_local_repo(&repo_name, "baseline")?;
    git(&repo_root, &["add", "."]);
    git(&repo_root, &["commit", "--no-gpg-sign", "-m", "baseline"]);
    let doc_id = state
        .repo
        .get_tracked_docid_in_local_repo(&repo_name, "note.md")?
        .expect("doc id");
    state.repo.append_generated_op_in_local_repo(
        &repo_name,
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: baseline.len() as u32,
                    content: "ledger\n".into(),
                },
                2,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )?;
    git(&repo_root, &["mv", "note.md", "renamed.md"]);
    std::fs::write(repo_root.join("renamed.md"), format!("{baseline}git import\n"))?;

    let report = apply_import(&state.repo, &repo_name, &repo_root)?;

    assert_eq!(report.applied, 1);
    let pending = state.repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0].path, "renamed.md");
    assert_eq!(pending[0].renamed_from.as_deref(), Some("note.md"));
    assert_eq!(pending[0].status, ChangeStatus::Renamed);
    assert!(pending[0].has_conflict, "{pending:?}");

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(repo_name.clone(), None);
    session.set_scope_nonce(Some(34));

    handle_resolve_conflict(
        &state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "renamed.md".into(),
            doc_id: Some(doc_id),
        },
        ConflictResolution::KeepFs,
    )
    .await;

    match uni_rx.recv().await.expect("conflict resolved") {
        ServerMessage::ConflictResolved {
            repo_id: actual_repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
            ..
        } => {
            assert_eq!(actual_repo_id, Some(repo_id));
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(34));
            assert_eq!(path, "renamed.md");
            assert_eq!(resolution, "KeepFs");
        }
        other => panic!("expected ConflictResolved, got {other:?}"),
    }
    let pending = state.repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert!(pending.is_empty(), "{pending:?}");
    let staged = state.repo.list_staged_in_local_repo(&repo_name)?;
    assert_eq!(staged.len(), 1, "{staged:?}");
    assert_eq!(staged[0].path, "renamed.md");
    assert_eq!(staged[0].renamed_from.as_deref(), Some("note.md"));
    assert_eq!(staged[0].status, ChangeStatus::Renamed);
    assert!(!staged[0].has_conflict, "{staged:?}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keep_fs_resolves_rename_pair_by_staging_all_related_entries() -> anyhow::Result<()> {
    let (dir, state, repo_id, _test_id) = build_state()?;
    let repo_name = state.repo.local_repo_name().to_string();
    write_workspace_file(&dir, &repo_name, "notes/a.md", "hello\n");
    state.repo.run_on_local_repo(&repo_name, |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello\n"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;
    state
        .repo
        .stage_pending_in_local_repo(&repo_name, "notes/a.md")?;
    state
        .repo
        .commit_staged_in_local_repo(&repo_name, "baseline")?;
    let doc_id = state
        .repo
        .get_tracked_docid_in_local_repo(&repo_name, "notes/a.md")?
        .expect("doc id");

    std::fs::remove_file(dir.path().join("vault").join(&repo_name).join("notes/a.md"))?;
    write_workspace_file(&dir, &repo_name, "notes/b.md", "hello\nrenamed\n");
    state.repo.run_on_local_repo(&repo_name, |db| {
        pending_fs::upsert_many(
            db,
            &[
                PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: Some(doc_id),
                    change_type: ChangeStatus::Deleted,
                    content_hash: String::new(),
                    detected_at: 2,
                    has_conflict: true,
                },
                PendingFsEntry {
                    path: "notes/b.md".into(),
                    renamed_from: Some("notes/a.md".into()),
                    doc_id: Some(doc_id),
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello\nrenamed\n"),
                    detected_at: 3,
                    has_conflict: true,
                },
            ],
        )
        .map(|_| ())
    })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo(repo_name.clone(), None);
    session.set_scope_nonce(Some(33));

    handle_resolve_conflict(
        &state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "notes/b.md".into(),
            doc_id: Some(doc_id),
        },
        ConflictResolution::KeepFs,
    )
    .await;

    match uni_rx.recv().await.expect("conflict resolved") {
        ServerMessage::ConflictResolved {
            repo_id: actual_repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
            ..
        } => {
            assert_eq!(actual_repo_id, Some(repo_id));
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(33));
            assert_eq!(path, "notes/b.md");
            assert_eq!(resolution, "KeepFs");
        }
        other => panic!("expected ConflictResolved, got {other:?}"),
    }
    let pending = state.repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert!(pending.is_empty(), "{pending:?}");
    let mut staged = state.repo.list_staged_in_local_repo(&repo_name)?;
    staged.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(staged.len(), 2, "{staged:?}");
    assert_eq!(staged[0].path, "notes/a.md");
    assert_eq!(staged[0].status, ChangeStatus::Deleted);
    assert!(!staged[0].has_conflict, "{staged:?}");
    assert_eq!(staged[1].path, "notes/b.md");
    assert_eq!(staged[1].renamed_from.as_deref(), Some("notes/a.md"));
    assert_eq!(staged[1].status, ChangeStatus::Added);
    assert!(!staged[1].has_conflict, "{staged:?}");
    Ok(())
}
