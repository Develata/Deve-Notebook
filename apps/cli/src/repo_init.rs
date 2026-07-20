//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 03_storage/projection#projection-locator-contract
//!   - 04_repository#repo-catalog-contract
//!
//! Shared local repo initialization path for CLI and server-created repos.

use deve_core::ledger::RepoManager;
use deve_core::ledger::init::RepoInitOptions;
use std::path::{Path, PathBuf};

pub(crate) struct LocalRepoInitReport {
    pub repo_name: String,
    pub repo_id: uuid::Uuid,
    pub workspace_root: PathBuf,
}

pub(crate) fn prepare_local_repo_workspace(
    ledger_dir: &Path,
    repo_id: uuid::Uuid,
    projection_base: &Path,
    snapshot_depth: usize,
    repo_url: Option<String>,
) -> anyhow::Result<LocalRepoInitReport> {
    let execution_name = repo_id.to_string();
    let repo = RepoManager::init_with_options(
        ledger_dir,
        snapshot_depth,
        Some(&execution_name),
        RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url,
        },
    )?;
    let actual_repo_name = repo.local_repo_name().to_string();
    let locator = repo.prepare_projection_locator_for_repo_creation(repo_id, projection_base)?;
    let workspace_root = locator.projection_base_abs.join(&locator.workspace_segment);
    std::fs::create_dir_all(&workspace_root)?;
    deve_core::utils::notegit::ensure_repo_identity_marker(
        &workspace_root,
        repo_id,
        &actual_repo_name,
    )?;
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&workspace_root)?;
    std::fs::create_dir_all(deve_core::utils::notegit::host_keys_dir(ledger_dir))?;

    Ok(LocalRepoInitReport {
        repo_name: actual_repo_name,
        repo_id,
        workspace_root,
    })
}

pub(crate) fn initialize_initial_local_repo_workspace(
    ledger_dir: &Path,
    initial_alias: &str,
    projection_base: &Path,
    snapshot_depth: usize,
    repo_id: Option<uuid::Uuid>,
    repo_url: Option<String>,
) -> anyhow::Result<LocalRepoInitReport> {
    let repo_id = repo_id.unwrap_or_else(uuid::Uuid::new_v4);
    let mut report = prepare_local_repo_workspace(
        ledger_dir,
        repo_id,
        projection_base,
        snapshot_depth,
        repo_url,
    )?;
    let repo = RepoManager::init_existing_for_repo_id(ledger_dir, snapshot_depth, repo_id)?;
    repo.seed_catalog_membership_from_records()?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let lifecycle_request_id = uuid::Uuid::new_v4();
    let prepared = repo.prepare_repo_creation_membership(repo_id, lifecycle_request_id)?;
    let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
    let permit = authority.permit(repo_id)?;
    repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
    let alias = repo
        .host_repo_alias_runtime()
        .set_alias(repo_id, initial_alias, 0)?
        .binding;
    report.repo_name = alias.alias;
    Ok(report)
}
