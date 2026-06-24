use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_conflict_keep_fs_resolves_to_clean_staged_entry() -> anyhow::Result<()> {
    let fixture = create_imported_conflict_fixture()?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(fixture.state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(fixture.repo_name.clone(), None);
    grant_browser_write(&fixture.state, &mut session, fixture.repo_id, 31)?;

    handle_resolve_conflict(
        &fixture.state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(fixture.doc_id),
        domain: None,
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
    session.switch_repo(fixture.repo_name.clone(), None);
    grant_browser_write(&fixture.state, &mut session, fixture.repo_id, 32)?;

    handle_resolve_conflict(
        &fixture.state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(fixture.doc_id),
        domain: None,
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
    state.repo.ensure_local_repo_workspace_identity(&repo_name)?;
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
                has_conflict: false,            },
        )
    })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(repo_name.clone(), None);
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(&repo_name))?
        .ok_or_else(|| anyhow::anyhow!("missing repo info"))?
        .uuid;
    grant_browser_write(&state, &mut session, repo_id, 35)?;

    handle_resolve_conflict(
        &state,
        &ch,
        &mut session,
        ScPathTarget {
            path: "note.md".into(),
            doc_id: None,
        domain: None,
        },
        ConflictResolution::KeepFs,
    )
    .await;

    match uni_rx.recv().await.expect("protocol error") {
        ServerMessage::ProtocolError {
            error, scope_nonce, ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScConflictTargetMissing);
            assert_eq!(
                error.detail.as_deref(),
                Some("Source control target is not a conflict: note.md")
            );
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
