//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::{
    AppState, channel::DualChannel, handlers::sync::SyncHelloInput, security, session::WsSession,
    tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::VersionVector;
use deve_core::protocol::{ServerError, ServerMessage, SessionProof};
use deve_core::security::IdentityKeyPair;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::mpsc;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_projection_base_for_all_local_repos(&vault);
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone())),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                identity_key.peer_id(),
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

pub(super) fn signed_hello(remote: &IdentityKeyPair, vector: &VersionVector) -> SyncHelloInput {
    let peer_id = remote.peer_id();
    let sorted_map: std::collections::BTreeMap<_, _> = vector.iter().collect();
    let vec_bytes = serde_json::to_vec(&sorted_map).expect("serialize vector");
    let mut msg = Vec::new();
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(peer_id.as_str().as_bytes());
    msg.extend_from_slice(&vec_bytes);
    SyncHelloInput {
        peer_id,
        peer_pubkey: remote.public_key_bytes().to_vec(),
        session_proof: SessionProof::new(remote.sign(&msg)),
        remote_vector: vector.clone(),
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 1,
    }
}

pub(super) fn signed_hello_for_repo(remote: &IdentityKeyPair, repo_id: uuid::Uuid) -> SyncHelloInput {
    let mut hello = signed_hello(remote, &VersionVector::new());
    hello.repo_id = repo_id;
    hello
}

pub(super) fn signed_hello_for_scope(
    remote: &IdentityKeyPair,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> SyncHelloInput {
    let mut hello = signed_hello_for_repo(remote, repo_id);
    hello.scope_nonce = scope_nonce;
    hello
}

pub(super) fn unicast_channel(state: &Arc<AppState>) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
    let (uni_tx, uni_rx) = mpsc::channel(16);
    (DualChannel::new(state.tx.clone(), uni_tx), uni_rx)
}

pub(super) fn empty_session() -> WsSession { WsSession::new() }

pub(super) fn block_shadow_peer_dir(state: &Arc<AppState>, remote: &IdentityKeyPair) -> anyhow::Result<()> {
    std::fs::create_dir_all(state.repo.remotes_dir())?;
    let path = state.repo.remotes_dir().join(remote.peer_id().to_filename());
    std::fs::write(path, b"blocked")?;
    Ok(())
}

pub(super) async fn collect_unicast_messages(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> anyhow::Result<Vec<ServerMessage>> {
    let first = rx.recv().await.expect("at least one message");
    let mut messages = vec![first];
    while let Ok(msg) = rx.try_recv() {
        messages.push(msg);
    }
    Ok(messages)
}

pub(super) async fn recv_protocol_error(rx: &mut mpsc::Receiver<ServerMessage>) -> ServerError {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => error,
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

pub(super) fn assert_runtime_binding_cleared(session: &WsSession) {
    assert!(session.bound_repo_id.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.writer_identity.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
}
