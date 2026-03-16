use super::repo_scope::resolve_session_repo_and_sync;
use super::{AppState, security, session::WsSession, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use deve_core::protocol::ServerErrorCode;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx: broadcast::channel(32).0,
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
    ))
}

#[test]
fn runtime_remote_scope_rejects_exact_selector_with_stale_uuid() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let first = uuid::Uuid::new_v4();
    let second = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: first,
            name: "wiki".into(),
            url: Some("urn:test:wiki-a".into()),
        },
    )?;
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: second,
            name: "wiki".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    )?;
    let first_selector = state
        .repo
        .find_remote_repo_selector_by_id(&peer_id, first)?
        .expect("selector for first repo");
    let second_selector = state
        .repo
        .find_remote_repo_selector_by_id(&peer_id, second)?
        .expect("selector for second repo");
    assert_ne!(first_selector, second_selector);
    let (selector, stale_uuid) = if first_selector != "wiki" {
        (first_selector, second)
    } else {
        (second_selector, first)
    };
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo(selector, Some(stale_uuid));

    let err = resolve_session_repo_and_sync(&state, &mut session)
        .expect_err("runtime remote scope must fail closed on stale uuid");
    let mapped = super::repo_scope::map_repo_scope_error(err);
    assert_eq!(mapped.code, ServerErrorCode::ScRepoContextInvalid);
    Ok(())
}
