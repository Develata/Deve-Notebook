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

pub(crate) fn initialize_local_repo_workspace(
    ledger_dir: &Path,
    repo_name: &str,
    projection_base: &Path,
    snapshot_depth: usize,
    repo_id: Option<uuid::Uuid>,
    repo_url: Option<String>,
) -> anyhow::Result<LocalRepoInitReport> {
    let repo = RepoManager::init_with_options(
        ledger_dir,
        snapshot_depth,
        Some(repo_name),
        RepoInitOptions { repo_id, repo_url },
    )?;
    let actual_repo_name = repo.local_repo_name().to_string();
    let info = repo
        .get_repo_info_for(None, Some(&actual_repo_name))?
        .ok_or_else(|| anyhow::anyhow!("Local repo metadata is missing for {actual_repo_name}"))?;
    repo.set_projection_base_for_local_repo(&actual_repo_name, projection_base)?;
    let workspace_root = repo.local_repo_workspace_root(&actual_repo_name)?;
    std::fs::create_dir_all(&workspace_root)?;
    repo.ensure_local_repo_workspace_identity(&actual_repo_name)?;
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&workspace_root)?;
    std::fs::create_dir_all(deve_core::utils::notegit::host_keys_dir(ledger_dir))?;

    Ok(LocalRepoInitReport {
        repo_name: info.name,
        repo_id: info.uuid,
        workspace_root,
    })
}
