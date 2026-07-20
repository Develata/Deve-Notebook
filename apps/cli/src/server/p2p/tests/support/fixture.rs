use crate::server::p2p::ExchangeStats;
use crate::server::{AppState, tree_state::RepoTreeRegistry};
use deve_core::config::{P2pPeerConfig, SyncMode};
use deve_core::models::{DocId, FactActor, LedgerEntry, Op, PeerId, VersionVector};
use deve_core::protocol::{ScopeNonce, ServerMessage};
use deve_core::security::{EncryptedOp, IdentityKeyPair};
use deve_core::sync::SyncManager;
use deve_core::sync::handshake_proof::sign_sync_hello;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;

pub(crate) const REMOTE_PEER_ID: &str = "bbbbbbbbbbbb";
pub(crate) const THIRD_PARTY_PEER_ID: &str = "aaaaaaaaaaaa";
pub(crate) const LOCAL_TARGET_PEER_ID: &str = "cccccccccccc";

pub(crate) fn test_state(identity: Arc<IdentityKeyPair>) -> anyhow::Result<Arc<AppState>> {
    Ok(test_state_with_dir(identity)?.1)
}

pub(crate) fn test_state_with_dir(
    identity: Arc<IdentityKeyPair>,
) -> anyhow::Result<(tempfile::TempDir, Arc<AppState>)> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let host_keys_dir = deve_core::utils::notegit::host_keys_dir(&ledger_dir);
    std::fs::create_dir_all(&host_keys_dir)?;
    std::fs::write(host_keys_dir.join("identity.key"), identity.to_bytes())?;
    let (repo, _repo_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger_dir,
        "default",
        &dir.path().join("vault"),
        10,
        Some("urn:default"),
    )?;
    let repo = Arc::new(repo);
    let (tx, _rx) = tokio::sync::broadcast::channel(8);
    let sync_manager = Arc::new(SyncManager::new_checked(repo.clone())?);
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager,
            tx,
            plugins: Vec::new(),
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                identity.peer_id(),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key: identity,
        }),
    ))
}

pub(crate) fn peer(repo_id: uuid::Uuid) -> P2pPeerConfig {
    peer_with_id(repo_id, REMOTE_PEER_ID)
}

pub(crate) fn peer_with_id(repo_id: uuid::Uuid, peer_id: &str) -> P2pPeerConfig {
    P2pPeerConfig {
        label: "peer-b".into(),
        peer_id: peer_id.into(),
        repo_id: repo_id.to_string(),
        ws_url: "ws://127.0.0.1:3002/ws".into(),
        auth_token_env: "DEVE_TEST_TOKEN".into(),
        enabled: true,
    }
}

pub(crate) fn dummy_payload() -> Vec<EncryptedOp> {
    vec![EncryptedOp {
        doc_id: None,
        peer_seq: 1_u64.into(),
        ciphertext: vec![1, 2, 3],
        nonce: vec![0; 12],
    }]
}

pub(crate) fn append_local_op(state: &Arc<AppState>, repo_id: uuid::Uuid) -> anyhow::Result<()> {
    state.sync_engine.get_or_create_strict(repo_id)?;
    let doc_id = DocId::new();
    state
        .repo
        .local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            state.repo.local_repo_name(),
            doc_id,
            Op::Insert {
                pos: 0,
                content: "local".into(),
            },
            1,
        )?;
    Ok(())
}

pub(crate) fn append_remote_shadow_op(
    state: &Arc<AppState>,
    repo_id: uuid::Uuid,
    remote_peer: &PeerId,
) -> anyhow::Result<()> {
    let doc_id = DocId::new();
    let entry = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 0,
            content: "remote-shadow".into(),
        },
        1,
        remote_peer.clone(),
        1,
        None,
        None,
    );
    state
        .repo
        .append_remote_ops(remote_peer, &repo_id, &[entry])?;
    Ok(())
}

pub(crate) fn authenticated_stats(peer_id: PeerId) -> ExchangeStats {
    ExchangeStats {
        saw_hello: true,
        authenticated_peer_id: Some(peer_id),
        ..Default::default()
    }
}

pub(crate) fn signed_server_hello(
    identity: &IdentityKeyPair,
    repo_id: uuid::Uuid,
    vector: VersionVector,
) -> ServerMessage {
    let peer_id = identity.peer_id();
    let signature = sign_sync_hello(identity, &vector).expect("version vector serializes");

    ServerMessage::SyncHello {
        peer_id,
        repo_id,
        scope_nonce: ScopeNonce::new(0),
        pub_key: identity.public_key_bytes().to_vec(),
        signature,
        vector,
    }
}
