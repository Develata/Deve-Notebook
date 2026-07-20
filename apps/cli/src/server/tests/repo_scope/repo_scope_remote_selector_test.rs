use super::repo_scope::resolve_session_repo_and_sync;
use super::{session::WsSession, tree_state::RepoTreeRegistry, AppState};
use crate::server::security;
use deve_core::config::SyncMode;
use deve_core::ledger::RepoInfo;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::broadcast;

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let (repo, _repo_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &dir.path().join("ledger"),
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(32);
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

#[test]
fn resolve_session_repo_keeps_exact_remote_selector_with_uuid() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let repo_id = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: "shadow-notes".into(),
            url: Some("urn:test:shadow-notes".into()),
        },
    )?;

    let exact_selector = state
        .repo
        .find_remote_repo_selector_by_id(&peer_id, repo_id)?
        .expect("remote selector");
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(exact_selector.clone(), Some(repo_id));

    let resolved = resolve_session_repo_and_sync(&state, &mut session)?;

    assert_eq!(resolved.branch, Some(peer_id));
    assert_eq!(resolved.repo_id, repo_id);
    assert_eq!(resolved.repo_name, exact_selector);
    assert_eq!(resolved.session_name, "shadow-notes");
    assert_eq!(session.active_repo.as_deref(), Some("shadow-notes"));
    Ok(())
}
