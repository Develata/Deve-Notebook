use super::support::recv_changes;
use super::*;
use deve_core::ledger::RepoInfo;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::commits::{self, COMMITS_ORDER_TABLE};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_commit_history_is_allowed() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: test_id,
            name: "shadow-notes".into(),
            url: Some("urn:test".into()),
        },
    )?;

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
async fn readonly_remote_history_repairs_legacy_missing_order_table() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: test_id,
            name: "shadow-notes".into(),
            url: Some("urn:test".into()),
        },
    )?;
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
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: test_id,
            name: "shadow-notes".into(),
            url: Some("urn:test".into()),
        },
    )?;
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
async fn remote_changes_without_repo_selection_clear_stale_db_and_sync_binding()
-> anyhow::Result<()> {
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
