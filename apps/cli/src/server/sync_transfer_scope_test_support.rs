//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
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
            search_service: None,
            identity_key,
        }),
        repo_id,
    ))
}

pub(super) fn append_local_doc(state: &Arc<AppState>) -> anyhow::Result<()> {
    let repo_name = state.repo.local_repo_name().to_string();
    let doc_id = DocId::new();
    state.repo.append_generated_op_in_local_repo(&repo_name, doc_id, PeerId::new("local"), |seq| {
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
    })?;
    Ok(())
}

pub(super) fn unicast_channel(
    state: &Arc<AppState>,
) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
    let (uni_tx, uni_rx) = mpsc::channel(8);
    (DualChannel::new(state.tx.clone(), uni_tx), uni_rx)
}

pub(super) fn bound_session(
    repo_id: uuid::Uuid,
    peer: Option<PeerId>,
    scope_nonce: Option<u64>,
) -> WsSession {
    let mut session = WsSession::new();
    if let Some(peer) = peer {
        session.set_authenticated(peer);
    }
    session.bind_repo(repo_id);
    if let Some(scope_nonce) = scope_nonce {
        session.set_sync_scope_nonce(scope_nonce);
    }
    session
}

pub(super) fn sync_range() -> Vec<(PeerId, (u64, u64))> {
    vec![(PeerId::new("test-peer"), (1, 2))]
}

pub(super) async fn recv_protocol_error(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> ServerError {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => error,
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

pub(super) async fn recv_sync_push_nonce(rx: &mut mpsc::Receiver<ServerMessage>) -> u64 {
    match rx.recv().await {
        Some(ServerMessage::SyncPush { scope_nonce, .. }) => scope_nonce,
        other => panic!("expected SyncPush, got {:?}", other),
    }
}

pub(super) async fn recv_sync_snapshot_nonce(rx: &mut mpsc::Receiver<ServerMessage>) -> u64 {
    match rx.recv().await {
        Some(ServerMessage::SyncPushSnapshot { scope_nonce, .. }) => scope_nonce,
        other => panic!("expected SyncPushSnapshot, got {:?}", other),
    }
}

pub(super) fn assert_sync_binding_cleared(session: &WsSession) {
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
}
