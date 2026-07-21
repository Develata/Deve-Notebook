//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 04_repository#repo-catalog-contract
//!
//! Durable-Normal catalog discovery and selector validation for local startup.

use super::super::RepoManager;
use super::super::init_reuse::should_reuse_existing_repo;
use super::super::manager::{LocalAuthorityDiscovery, RepoCatalogMembershipRecord};
use crate::utils::fs::checked_exists;
use anyhow::Result;
use redb::Database;
use std::collections::HashMap;
use std::path::Path;

pub(super) struct ExistingLocalRepo {
    pub(super) path: std::path::PathBuf,
    pub(super) info: super::super::RepoInfo,
}

pub(super) fn scan_cataloged_local_repos(
    local_dir: &Path,
    discovery: &LocalAuthorityDiscovery,
    records: &[RepoCatalogMembershipRecord],
) -> Result<Vec<ExistingLocalRepo>> {
    let mut repos = Vec::with_capacity(records.len());
    let mut urls = HashMap::new();
    for record in records {
        let repo_id = record.repo_id();
        let stem = repo_id.to_string();
        let path = local_dir.join(format!("{stem}.redb"));
        if !checked_exists(&path, "durable Normal local RepoId during init")? {
            anyhow::bail!(
                "Durable Normal local RepoId {} has no canonical authority database",
                repo_id
            );
        }
        let lease = discovery.lease(repo_id).map_err(|error| {
            anyhow::anyhow!(
                "Broken local repo {} while initializing durable catalog: {}",
                repo_id,
                error
            )
        })?;
        let info = RepoManager::read_local_repo_info_from_db(lease.db())?.ok_or_else(|| {
            anyhow::anyhow!(
                "Broken local repo {} while initializing durable catalog: repository metadata missing",
                repo_id
            )
        })?;
        if info.uuid != repo_id || info.name != stem {
            anyhow::bail!(
                "Broken local repo {} while initializing durable catalog: metadata identity does not match canonical RepoId",
                repo_id
            );
        }
        validate_current_cataloged_identity(
            local_dir.parent().expect("local has ledger parent"),
            record,
            lease.db(),
        )?;
        if let Some(url) = &info.url
            && let Some(owner) = urls.insert(url.clone(), stem.clone())
        {
            anyhow::bail!(
                "Broken local catalog: duplicate repository URL {} at {} and {}",
                url,
                owner,
                stem
            );
        }
        repos.push(ExistingLocalRepo { path, info });
    }
    Ok(repos)
}

pub(super) fn validate_current_cataloged_identity(
    ledger_dir: &Path,
    record: &RepoCatalogMembershipRecord,
    db: &Database,
) -> Result<()> {
    // The catalog digest is the immutable creation-time snapshot and includes
    // projection base/locator epoch. Explicit relocation is legal, so startup
    // validates the current DB/locator/marker tuple for internal consistency
    // without comparing it to that historical digest.
    super::super::manager::prepared_identity_for_existing_database(
        ledger_dir,
        record.repo_id(),
        db,
    )?;
    Ok(())
}

pub(super) fn select_existing_local_repo<'a>(
    repos: &'a [ExistingLocalRepo],
    requested_name: &str,
    requested_url: Option<&str>,
    requested_id: Option<uuid::Uuid>,
) -> Result<Option<&'a ExistingLocalRepo>> {
    if let Some(repo_id) = requested_id {
        if let Some(repo) = repos.iter().find(|repo| repo.info.uuid == repo_id) {
            if repo.info.name != requested_name
                || !should_reuse_existing_repo(requested_url, &repo.info)
            {
                anyhow::bail!(
                    "Existing local RepoId {} metadata does not match explicit init request",
                    repo_id
                );
            }
            return Ok(Some(repo));
        }
        if let Some(repo) = repos.iter().find(|repo| {
            repo.info.name == requested_name
                && should_reuse_existing_repo(requested_url, &repo.info)
        }) {
            anyhow::bail!(
                "explicit repo-id init fails closed: repository selector {} resolves to existing RepoId {}, not requested RepoId {}",
                requested_name,
                repo.info.uuid,
                repo_id
            );
        }
        return Ok(None);
    }
    let exact_requested_id = uuid::Uuid::parse_str(requested_name)
        .ok()
        .filter(|repo_id| repo_id.to_string() == requested_name);
    let matches = repos
        .iter()
        .filter(|repo| match requested_url {
            Some(_) => should_reuse_existing_repo(requested_url, &repo.info),
            None => exact_requested_id == Some(repo.info.uuid) || repos.len() == 1,
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [repo] => Ok(Some(*repo)),
        _ => anyhow::bail!(
            "Ambiguous local repository init selector {} matched {} RepoIds; display names are host-local aliases, so pass an explicit RepoId or unique URL",
            requested_name,
            matches.len()
        ),
    }
}
