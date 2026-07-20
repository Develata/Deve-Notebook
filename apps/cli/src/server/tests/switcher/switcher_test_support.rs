//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::{
    channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry, AppState,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::protocol::ServerMessage;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::{broadcast, mpsc};

pub(crate) fn browser_session(scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(scope_nonce));
    session
}

pub(crate) fn app_state(
    repo: RepoManager,
    _projection_base: PathBuf,
    host_dir: PathBuf,
) -> anyhow::Result<Arc<AppState>> {
    app_state_with_tree(
        repo,
        _projection_base,
        host_dir,
        Arc::new(RepoTreeRegistry::new()),
    )
}

pub(crate) fn app_state_with_tree(
    repo: RepoManager,
    _projection_base: PathBuf,
    host_dir: PathBuf,
    tree_manager: Arc<RepoTreeRegistry>,
) -> anyhow::Result<Arc<AppState>> {
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&host_dir)?;
    Ok(Arc::new(AppState {
        repo: repo.clone(),
        sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
        tx,
        plugins: vec![],
        sync_engine: Arc::new(RepoScopedSyncEngine::new(
            identity_key.peer_id(),
            repo,
            SyncMode::Auto,
        )),
        tree_manager,
        #[cfg(feature = "search")]
        search_available: false,
        identity_key,
    }))
}

pub(crate) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let (repo, _repo_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &dir.path().join("ledger"),
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let state = app_state(repo, projection_base, dir.path().join("host"))?;
    Ok((dir, state))
}

pub(crate) fn unicast_channel(
    state: &Arc<AppState>,
) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
    let (uni_tx, uni_rx) = mpsc::channel(8);
    (DualChannel::new(state.tx.clone(), uni_tx), uni_rx)
}
