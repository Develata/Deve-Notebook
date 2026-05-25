//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use crate::server::{AppState, security, session::WsSession, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
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
    ))
}

pub(super) fn bind_stale_shadow_scope(
    state: &Arc<AppState>,
    session: &mut WsSession,
    repo_id: uuid::Uuid,
    nonce: u64,
) -> anyhow::Result<()> {
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), Some(repo_id));
    session.set_active_db(
        state
            .repo
            .open_database(None, state.repo.local_repo_name())?,
    );
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(repo_id);
    session.set_sync_scope_nonce(nonce);
    Ok(())
}

pub(super) fn seed_shadow_repo(state: &Arc<AppState>) -> anyhow::Result<(PeerId, uuid::Uuid)> {
    let shadow_peer = PeerId::new("peer-a");
    let shadow_repo = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &shadow_peer,
        &RepoInfo {
            uuid: shadow_repo,
            name: shadow_repo.to_string(),
            url: Some(format!("urn:shadow:{shadow_repo}")),
        },
    )?;
    Ok((shadow_peer, shadow_repo))
}
