//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerError, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
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
            search_available: false,
            identity_key,
        }),
        repo_id,
    ))
}

pub(super) fn unicast_channel(
    state: &Arc<AppState>,
) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
    let (uni_tx, uni_rx) = mpsc::channel(8);
    (DualChannel::new(state.tx.clone(), uni_tx), uni_rx)
}

pub(super) fn stale_unbound_session(
    state: &Arc<AppState>,
    remote_branch: bool,
    scope_nonce: u64,
) -> anyhow::Result<WsSession> {
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let mut session = WsSession::new();
    if remote_branch {
        session.switch_branch(Some("peer-a".into()));
    }
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(scope_nonce);
    Ok(session)
}

pub(super) fn browser_session_without_sync_scope(
    state: &Arc<AppState>,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> anyhow::Result<WsSession> {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("default".into(), Some(repo_id));
    session.set_scope_nonce(Some(scope_nonce));
    session.set_active_db(state.repo.open_database(None, state.repo.local_repo_name())?);
    session.set_authenticated(PeerId::new("browser"));
    session.bind_repo(repo_id);
    Ok(session)
}

pub(super) async fn recv_protocol_error(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (ServerError, Option<u64>) {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => (error, scope_nonce),
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

pub(super) fn try_recv_protocol_error(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (ServerError, Option<u64>) {
    match rx.try_recv() {
        Ok(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => (error, scope_nonce),
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

pub(super) fn assert_runtime_binding_cleared(session: &WsSession) {
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
}
