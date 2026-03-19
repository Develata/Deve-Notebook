use super::resolve_target_repos;
use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let second = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:notes"))?;
    let second_id = second.get_repo_info()?.expect("notes info").uuid;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
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
            search_service: None,
            identity_key,
        }),
        second_id,
    ))
}

#[test]
fn repair_node_check_requires_explicit_repo_selector() -> anyhow::Result<()> {
    let (_dir, state, _notes_id) = build_state()?;
    let err = resolve_target_repos(
        state.as_ref(),
        &RepoSelector {
            repo_name: None,
            repo_id: None,
        },
        true,
    )
    .expect_err("repair node-check must fail closed without selector");
    assert!(
        err.to_string()
            .contains("Repository selector required for repair node-check")
    );
    Ok(())
}

#[test]
fn readonly_node_check_keeps_all_local_repos() -> anyhow::Result<()> {
    let (_dir, state, _notes_id) = build_state()?;
    let repos = resolve_target_repos(
        state.as_ref(),
        &RepoSelector {
            repo_name: None,
            repo_id: None,
        },
        false,
    )?;
    assert_eq!(repos, vec!["default".to_string(), "notes".to_string()]);
    Ok(())
}

#[test]
fn repair_node_check_accepts_exact_repo_selector() -> anyhow::Result<()> {
    let (_dir, state, notes_id) = build_state()?;
    let repos = resolve_target_repos(
        state.as_ref(),
        &RepoSelector {
            repo_name: Some("notes".into()),
            repo_id: Some(notes_id),
        },
        true,
    )?;
    assert_eq!(repos, vec!["notes".to_string()]);
    Ok(())
}
