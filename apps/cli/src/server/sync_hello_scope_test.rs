use super::handlers::sync::{SyncHelloInput, handle_sync_hello};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::VersionVector;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::security::IdentityKeyPair;
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
                identity_key.peer_id(),
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

fn signed_hello(remote: &IdentityKeyPair, repo_id: uuid::Uuid) -> SyncHelloInput {
    let vector = VersionVector::new();
    let peer_id = remote.peer_id();
    let sorted_map: std::collections::BTreeMap<_, _> = vector.iter().collect();
    let vec_bytes = serde_json::to_vec(&sorted_map).expect("serialize vector");
    let mut msg = Vec::new();
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(peer_id.as_str().as_bytes());
    msg.extend_from_slice(&vec_bytes);
    SyncHelloInput {
        peer_id,
        pub_key: remote.public_key_bytes().to_vec(),
        signature: remote.sign(&msg),
        remote_vector: vector,
        repo_id,
        scope_nonce: 1,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_non_browser_active_branch_peer_mismatch() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let current_peer = IdentityKeyPair::generate();
    let incoming_peer = IdentityKeyPair::generate();
    let hello = signed_hello(&incoming_peer, repo_id);
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(current_peer.peer_id().to_string()));
    session.set_authenticated(current_peer.peer_id());
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(3);

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("active_branch")),
                "unexpected detail: {:?}",
                error.detail
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.bound_repo_id.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.active_repo.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}
