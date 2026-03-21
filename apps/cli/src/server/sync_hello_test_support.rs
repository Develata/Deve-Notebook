use super::super::handlers::sync::SyncHelloInput;
use super::super::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::protocol::ServerMessage;
use deve_core::security::IdentityKeyPair;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use deve_core::sync::vector::VersionVector;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::mpsc;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let (tx, _rx) = tokio::sync::broadcast::channel(16);
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
        pub_key: remote.public_key_bytes().to_vec(),
        signature: remote.sign(&msg),
        remote_vector: vector.clone(),
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 1,
    }
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
