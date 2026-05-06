use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_commit_history_is_allowed() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_commit_history(&state, &ch, &mut session, "req-1".into(), 10).await;
    let (repo_id, first_message) = recv_history(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(first_message, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_commit_diff_is_allowed() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let doc_id = DocId::new();
    state.repo.append_remote_op(
        &peer_id,
        &test_id,
        &shadow_create_file(&peer_id, doc_id, "note.md", 1),
    )?;
    let first_ledger_seq = state.repo.append_remote_op(
        &peer_id,
        &test_id,
        &shadow_insert(&peer_id, doc_id, 0, "hello", 2),
    )?;
    let first = create_shadow_commit(
        state.repo.as_ref(),
        &peer_id,
        &test_id,
        "first",
        first_ledger_seq,
    )?;
    let second_ledger_seq = state.repo.append_remote_op(
        &peer_id,
        &test_id,
        &shadow_insert(&peer_id, doc_id, 5, " remote", 3),
    )?;
    let second = create_shadow_commit(
        state.repo.as_ref(),
        &peer_id,
        &test_id,
        "second",
        second_ledger_seq,
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(17));

    handle_get_commit_diff(
        &state,
        &ch,
        &mut session,
        "req-1".into(),
        Some(first.id),
        second.id,
    )
    .await;
    let (repo_id, branch, scope_nonce, diffs) = recv_commit_diff(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(branch, Some(peer_id));
    assert_eq!(scope_nonce, Some(17));
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, Some(doc_id));
    assert_eq!(diffs[0].path, "note.md");
    assert_eq!(diffs[0].status, ChangeStatus::Modified);
    assert_eq!(diffs[0].old_content, "hello");
    assert_eq!(diffs[0].new_content, "hello remote");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_commit_diff_reports_rename_projection() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let doc_id = DocId::new();
    state.repo.append_remote_op(
        &peer_id,
        &test_id,
        &shadow_create_file(&peer_id, doc_id, "note.md", 1),
    )?;
    let first_ledger_seq = state.repo.append_remote_op(
        &peer_id,
        &test_id,
        &shadow_insert(&peer_id, doc_id, 0, "hello", 2),
    )?;
    let first = create_shadow_commit(
        state.repo.as_ref(),
        &peer_id,
        &test_id,
        "first",
        first_ledger_seq,
    )?;
    let second_ledger_seq = state.repo.append_remote_op(
        &peer_id,
        &test_id,
        &shadow_rename(&peer_id, doc_id, "renamed.md", 3),
    )?;
    let second = create_shadow_commit(
        state.repo.as_ref(),
        &peer_id,
        &test_id,
        "rename",
        second_ledger_seq,
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));
    session.set_scope_nonce(Some(19));

    handle_get_commit_diff(
        &state,
        &ch,
        &mut session,
        "req-rename".into(),
        Some(first.id),
        second.id,
    )
    .await;
    let (repo_id, branch, scope_nonce, diffs) = recv_commit_diff(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(branch, Some(peer_id));
    assert_eq!(scope_nonce, Some(19));
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, Some(doc_id));
    assert_eq!(diffs[0].previous_path.as_deref(), Some("note.md"));
    assert_eq!(diffs[0].path, "renamed.md");
    assert_eq!(diffs[0].status, ChangeStatus::Renamed);
    assert_eq!(diffs[0].old_content, "hello");
    assert_eq!(diffs[0].new_content, "hello");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_history_repairs_legacy_missing_order_table() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    state
        .repo
        .run_on_shadow_repo_by_id(&peer_id, &test_id, |db| {
            let _first = commits::create(db, "first", 1, 1)?;
            let _second = commits::create(db, "second", 1, 2)?;
            let write_txn = db.begin_write()?;
            let _ = write_txn.delete_table(COMMITS_ORDER_TABLE)?;
            write_txn.commit()?;
            Ok::<(), anyhow::Error>(())
        })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_commit_history(&state, &ch, &mut session, "req-1".into(), 10).await;
    let (repo_id, first_message) = recv_history(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(first_message.as_deref(), Some("second"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_changes_are_allowed_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = ensure_shadow_repo(state.repo.as_ref(), test_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;
    let (repo_id, paths) = recv_changes(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert!(paths.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_changes_without_repo_selection_clear_stale_db_and_sync_binding() -> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("peer-a".into()));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(13);

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Remote branch not available:"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
