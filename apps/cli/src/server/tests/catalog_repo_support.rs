//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-selector-resolution-contract
//!
//! Shared catalog-membership fixtures for `server` test harnesses.
//!
//! Repo listing/resolution is durable-membership backed: a bare
//! `RepoManager::init` repo carries no membership record, so
//! `is_local_repo_removed` treats it as removed and every selector resolution
//! fails closed with `Repository not found`. These helpers mirror the
//! production creation choreography in
//! `repo_init::{initialize_initial_local_repo_workspace, prepare_local_repo_workspace}`:
//! prepared projection locator + workspace identity marker + committed `Normal`
//! catalog membership. Machine names are canonical UUID strings; display
//! aliases are host-local and never consulted by resolution, so tests must
//! select repos by their UUID stem (`repo_id.to_string()` /
//! `repo.local_repo_name()`), not by alias.

use crate::repo_init::{initialize_initial_local_repo_workspace, prepare_local_repo_workspace};
use deve_core::ledger::RepoManager;
use std::path::Path;

/// Creates the initial cataloged local repo and returns an open handle plus its
/// canonical `RepoId`. `alias`/`repo_url` mirror the options a bare
/// `RepoManager::init(.., Some(alias), repo_url)` used to pass; the initial
/// membership is committed inside `initialize_initial_local_repo_workspace`.
pub(crate) fn catalog_initial_repo(
    ledger_dir: &Path,
    alias: &str,
    projection_base: &Path,
    snapshot_depth: usize,
    repo_url: Option<&str>,
) -> anyhow::Result<(RepoManager, uuid::Uuid)> {
    let repo_id = uuid::Uuid::new_v4();
    initialize_initial_local_repo_workspace(
        ledger_dir,
        alias,
        projection_base,
        snapshot_depth,
        Some(repo_id),
        repo_url.map(str::to_string),
    )?;
    let repo = RepoManager::init_existing_for_repo_id(ledger_dir, snapshot_depth, repo_id)?;
    repo.seed_catalog_membership_from_records()?;
    Ok((repo, repo_id))
}

/// Catalogs an additional local repo in the same ledger, using an
/// already-seeded `repo` handle (typically the one returned by
/// [`catalog_initial_repo`]) to claim cut authority and commit the new
/// membership. Returns the new `RepoId`.
pub(crate) fn catalog_additional_repo(
    repo: &RepoManager,
    ledger_dir: &Path,
    alias: &str,
    projection_base: &Path,
    snapshot_depth: usize,
    repo_url: Option<&str>,
) -> anyhow::Result<uuid::Uuid> {
    let repo_id = uuid::Uuid::new_v4();
    prepare_local_repo_workspace(
        ledger_dir,
        repo_id,
        projection_base,
        snapshot_depth,
        repo_url.map(str::to_string),
    )?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let prepared = repo.prepare_repo_creation_membership(repo_id, uuid::Uuid::new_v4())?;
    let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
    let permit = authority.permit(repo_id)?;
    repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
    repo.host_repo_alias_runtime()
        .set_alias(repo_id, alias, 0)?;
    Ok(repo_id)
}
