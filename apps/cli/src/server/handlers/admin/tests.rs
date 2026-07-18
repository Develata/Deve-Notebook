use super::{classify_admin_error, resolve_target_repos};
use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use axum::http::StatusCode;
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
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let mut second = RepoManager::init(&ledger, 10, Some("notes"), Some("urn:notes"))?;
    second.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let second_id = second.get_repo_info()?.expect("notes info").uuid;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
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
    assert_eq!(repos, vec![notes_id.to_string()]);
    Ok(())
}

#[test]
fn classify_admin_error_marks_broken_local_repo_as_internal_server_error() {
    let status = classify_admin_error(
        "Broken local repo notes while validating catalog: repository URL missing",
        StatusCode::BAD_REQUEST,
    );
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn classify_admin_error_marks_local_selector_mismatch_as_conflict() {
    let status = classify_admin_error(
        "Local repository selector not resolved for notes",
        StatusCode::BAD_REQUEST,
    );
    assert_eq!(status, StatusCode::CONFLICT);
}
