//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

pub(super) use super::key_exchange_message_test_support::{
    assert_key_provide, recv_key_denied, recv_protocol_error,
};

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, RepoId)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let host_dir = dir.path().join("host");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let repo = Arc::new(repo);
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx: broadcast::channel(8).0,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key: security::load_or_generate_identity_key(&host_dir)?,
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

pub(super) fn browser_session(scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

pub(super) fn bound_browser_session(repo_id: RepoId, scope_nonce: u64) -> WsSession {
    let mut session = browser_session(scope_nonce);
    session.switch_repo("notes".into(), Some(repo_id));
    session.bind_repo(repo_id);
    session
}

pub(super) fn remote_browser_session(
    peer_id: &PeerId,
    repo_id: RepoId,
    scope_nonce: u64,
) -> WsSession {
    let mut session = browser_session(scope_nonce);
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(repo_id));
    session.bind_repo(repo_id);
    session
}

pub(super) fn ensure_shadow_notes(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: RepoId,
    url: &str,
) -> anyhow::Result<()> {
    state.repo.ensure_shadow_repo_info(
        peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "shadow-notes".into(),
            url: Some(url.into()),
        },
    )
}
