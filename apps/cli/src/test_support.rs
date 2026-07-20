//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 03_storage/projection#projection-locator-contract
//!
//! Shared test fixture for catalog-backed local repos.
//!
//! Mirrors the production creation choreography in
//! `repo_init::initialize_initial_local_repo_workspace`: prepared locator +
//! workspace identity marker + durable catalog membership commit. Catalog
//! listing and selector resolution are membership-record backed, so a bare
//! `RepoManager::init` repo is invisible to every resolution path.

use deve_core::ledger::RepoManager;
use deve_core::models::RepoId;
use std::path::{Path, PathBuf};

pub(crate) struct CatalogedTestRepo {
    pub(crate) repo: RepoManager,
    pub(crate) repo_id: RepoId,
    pub(crate) workspace_root: PathBuf,
}

/// Creates a fully cataloged local repo with a UUID-canonical machine name,
/// a prepared projection locator under `projection_base`, a workspace with
/// identity marker, and a committed `Normal` catalog membership record.
pub(crate) fn init_cataloged_repo(
    ledger_dir: &Path,
    projection_base: &Path,
    snapshot_depth: usize,
) -> anyhow::Result<CatalogedTestRepo> {
    init_cataloged_repo_with_url(ledger_dir, projection_base, snapshot_depth, None)
}

/// Same choreography as [`init_cataloged_repo`], but binds an explicit repo
/// URL. Used by resolution fixtures that key on `RepoInfo::url`.
pub(crate) fn init_cataloged_repo_with_url(
    ledger_dir: &Path,
    projection_base: &Path,
    snapshot_depth: usize,
    repo_url: Option<String>,
) -> anyhow::Result<CatalogedTestRepo> {
    let repo_id = RepoId::new_v4();
    let report = crate::repo_init::prepare_local_repo_workspace(
        ledger_dir,
        repo_id,
        projection_base,
        snapshot_depth,
        repo_url,
    )?;
    let repo = RepoManager::init_existing_for_repo_id(ledger_dir, snapshot_depth, repo_id)?;
    repo.seed_catalog_membership_from_records()?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let prepared = repo.prepare_repo_creation_membership(repo_id, uuid::Uuid::new_v4())?;
    let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
    let permit = authority.permit(repo_id)?;
    repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
    Ok(CatalogedTestRepo {
        repo,
        repo_id,
        workspace_root: report.workspace_root,
    })
}
