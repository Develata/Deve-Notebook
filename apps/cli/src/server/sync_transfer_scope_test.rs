use super::handlers::sync::{handle_sync_request, handle_sync_snapshot_request};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key,
        }),
        repo_id,
    ))
}

fn append_local_doc(state: &Arc<AppState>) -> anyhow::Result<()> {
    let repo_name = state.repo.local_repo_name().to_string();
    let doc_id = DocId::new();
    state.repo.append_generated_op_in_local_repo(
        &repo_name,
        doc_id,
        PeerId::new("local"),
        move |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_sync_request_uses_bound_sync_scope_nonce_for_push() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.set_authenticated(PeerId::new("peer-a"));
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(17);

    handle_sync_request(
        &state,
        &ch,
        &mut session,
        repo_id,
        vec![(PeerId::new("test-peer"), (1, 2))],
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::SyncPush { scope_nonce, .. }) => assert_eq!(scope_nonce, 17),
        other => panic!("expected SyncPush, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_snapshot_request_uses_bound_sync_scope_nonce_for_push() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.set_authenticated(PeerId::new("peer-a"));
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(19);

    handle_sync_snapshot_request(&state, &ch, &mut session, PeerId::new("peer-a"), repo_id).await;

    match uni_rx.recv().await {
        Some(ServerMessage::SyncPushSnapshot { scope_nonce, .. }) => {
            assert_eq!(scope_nonce, 19)
        }
        other => panic!("expected SyncPushSnapshot, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_sync_request_rejects_missing_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.set_authenticated(PeerId::new("peer-a"));
    session.bind_repo(repo_id);

    handle_sync_request(
        &state,
        &ch,
        &mut session,
        repo_id,
        vec![(PeerId::new("test-peer"), (1, 2))],
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(error.detail.as_deref(), Some("sync scope nonce not bound"));
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_browser_sync_request_rejects_missing_authenticated_peer_even_when_repo_bound()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(23);

    handle_sync_request(
        &state,
        &ch,
        &mut session,
        repo_id,
        vec![(PeerId::new("test-peer"), (1, 2))],
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
