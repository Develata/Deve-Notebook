use super::{classify_admin_error, resolve_target_repos};
use crate::server::{AppState, security, tree_state::RepoTreeRegistry};
use axum::http::StatusCode;
use deve_core::config::SyncMode;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::broadcast;

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, uuid::Uuid)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let default = crate::test_support::init_cataloged_repo_with_url(
        &ledger,
        &projection_base,
        10,
        Some("urn:default".to_string()),
    )?;
    let default_id = default.repo_id;
    let second_id = crate::server::catalog_repo_support::catalog_additional_repo(
        &default.repo,
        &ledger,
        "notes",
        &projection_base,
        10,
        Some("urn:notes"),
    )?;
    let repo = Arc::new(default.repo);
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
        default_id,
        second_id,
    ))
}

#[test]
fn repair_node_check_requires_explicit_repo_selector() -> anyhow::Result<()> {
    let (_dir, state, _default_id, _notes_id) = build_state()?;
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
    let (_dir, state, default_id, notes_id) = build_state()?;
    let mut repos = resolve_target_repos(
        state.as_ref(),
        &RepoSelector {
            repo_name: None,
            repo_id: None,
        },
        false,
    )?;
    repos.sort();
    let mut expected = vec![default_id.to_string(), notes_id.to_string()];
    expected.sort();
    assert_eq!(repos, expected);
    Ok(())
}

#[test]
fn repair_node_check_accepts_exact_repo_selector() -> anyhow::Result<()> {
    let (_dir, state, _default_id, notes_id) = build_state()?;
    let repos = resolve_target_repos(
        state.as_ref(),
        &RepoSelector {
            repo_name: Some(notes_id.to_string()),
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
