//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use crate::server::{AppState, security, session::WsSession, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo_id = repo.get_repo_info()?.expect("default info").uuid;
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

pub(super) fn seed_stale_binding(
    session: &mut WsSession,
    state: &Arc<AppState>,
    repo_id: uuid::Uuid,
) {
    session.set_active_db(
        state
            .repo
            .open_database(None, state.repo.local_repo_name())
            .expect("local handle"),
    );
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(7);
}

pub(super) fn seed_remote_branch(state: &Arc<AppState>, peer_id: &PeerId, repo_id: uuid::Uuid) {
    state
        .repo
        .ensure_shadow_repo_info(
            peer_id,
            &RepoInfo {
                uuid: repo_id,
                name: "shadow-notes".into(),
                url: Some("urn:test:shadow-notes".into()),
            },
        )
        .expect("seed remote branch");
}
