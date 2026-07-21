//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract
//!   - 04_repository#repo-catalog-contract

use super::{ProjectionLocatorRecord, file_validation};
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::RepoId;
use anyhow::{Context, Result, anyhow};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

pub(super) fn validate_projection_locator_records(
    repo: &RepoManager,
    records: &[ProjectionLocatorRecord],
    require_all_local_locators: bool,
) -> Result<()> {
    let mut local_infos = Vec::new();
    for repo_name in repo.list_local_repo_names_for_execution()? {
        let info = repo
            .get_repo_info_for(None, Some(&repo_name))?
            .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))?;
        local_infos.push((repo_name, info));
    }
    validate_records_against_local_infos(repo, records, require_all_local_locators, local_infos)
}

pub(super) fn validate_projection_locator_records_for_prepared_creation(
    repo: &RepoManager,
    records: &[ProjectionLocatorRecord],
    prepared_repo_id: RepoId,
    prepared_info: RepoInfo,
) -> Result<()> {
    if repo
        .repo_catalog_membership_record(prepared_repo_id)?
        .is_some()
    {
        return Err(anyhow!(
            "Prepared Projection Locator target already has catalog state: {prepared_repo_id}"
        ));
    }
    let mut local_infos = Vec::new();
    for repo_name in repo.list_local_repo_names_for_execution()? {
        let info = repo
            .get_repo_info_for(None, Some(&repo_name))?
            .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))?;
        local_infos.push((repo_name, info));
    }
    let execution_name = prepared_repo_id.to_string();
    if prepared_info.uuid != prepared_repo_id || prepared_info.name != execution_name {
        return Err(anyhow!(
            "Prepared Projection Locator target must use canonical RepoId identity"
        ));
    }
    local_infos.push((execution_name, prepared_info));
    validate_records_against_local_infos(repo, records, false, local_infos)
}

fn validate_records_against_local_infos(
    repo: &RepoManager,
    records: &[ProjectionLocatorRecord],
    require_all_local_locators: bool,
    local_infos: Vec<(String, RepoInfo)>,
) -> Result<()> {
    resolved_roots_against_local_infos(repo, records, require_all_local_locators, local_infos)
        .map(|_| ())
}

struct ResolvedRoot {
    repo_id: RepoId,
    path: PathBuf,
    normalized_key: String,
}

fn resolved_roots_against_local_infos(
    repo: &RepoManager,
    records: &[ProjectionLocatorRecord],
    require_all_local_locators: bool,
    local_infos: Vec<(String, RepoInfo)>,
) -> Result<Vec<ResolvedRoot>> {
    file_validation::validate_projection_locator_file_shape(records)?;
    let records_by_id = records
        .iter()
        .map(|record| (record.repo_id, record))
        .collect::<HashMap<_, _>>();
    let local_infos_by_id = local_infos
        .iter()
        .map(|(_, info)| info.uuid)
        .collect::<HashSet<_>>();

    let mut roots = Vec::new();
    for (repo_name, info) in local_infos {
        let Some(record) = records_by_id.get(&info.uuid) else {
            if require_all_local_locators {
                return Err(anyhow!(
                    "Projection Locator missing for local repo {}",
                    info.name
                ));
            }
            continue;
        };
        roots.push(resolve_record_root(repo, &repo_name, info.uuid, record)?);
    }
    // Unknown RepoIds are prepared create truth, not normal catalog members.
    // They remain structurally validated and participate in workspace conflict
    // detection, but they must not make healthy cataloged repos unreadable.
    for record in records {
        if !local_infos_by_id.contains(&record.repo_id) {
            roots.push(resolve_record_root(
                repo,
                &record.repo_id.to_string(),
                record.repo_id,
                record,
            )?);
        }
    }

    for idx in 0..roots.len() {
        for other in (idx + 1)..roots.len() {
            let left = &roots[idx];
            let right = &roots[other];
            if left.normalized_key == right.normalized_key {
                return Err(anyhow!(
                    "Projection workspace conflict: repos {} and {} resolve to {:?}",
                    left.repo_id,
                    right.repo_id,
                    left.path
                ));
            }
            if left.path.starts_with(&right.path) || right.path.starts_with(&left.path) {
                return Err(anyhow!(
                    "Projection workspace nesting conflict between {:?} and {:?}",
                    left.path,
                    right.path
                ));
            }
        }
    }
    Ok(roots)
}

fn resolve_record_root(
    repo: &RepoManager,
    repo_name: &str,
    repo_id: RepoId,
    record: &ProjectionLocatorRecord,
) -> Result<ResolvedRoot> {
    let ledger_dir =
        std::fs::canonicalize(&repo.ledger_dir).unwrap_or_else(|_| repo.ledger_dir.clone());
    let projection_base_abs =
        std::fs::canonicalize(&record.projection_base_abs).with_context(|| {
            format!(
                "Failed to canonicalize Projection Locator base for {}: {:?}",
                repo_name, record.projection_base_abs
            )
        })?;
    let workspace_segment = &record.workspace_segment;
    let root = projection_base_abs.join(workspace_segment);
    if root.starts_with(&ledger_dir) {
        return Err(anyhow!(
            "Projection workspace for {} must not be inside ledger_dir: {:?}",
            repo_name,
            root
        ));
    }
    if root
        .components()
        .any(|component| matches!(component, Component::Normal(name) if name == ".notegit" || name == ".git"))
    {
        return Err(anyhow!(
            "Projection workspace for {} must not be inside .notegit or .git: {:?}",
            repo_name,
            root
        ));
    }
    Ok(ResolvedRoot {
        repo_id,
        path: root,
        normalized_key: normalized_workspace_key(&projection_base_abs, workspace_segment),
    })
}

pub(super) fn normalized_workspace_key(base: &Path, workspace_segment: &str) -> String {
    crate::utils::path::path_to_forward_slash(&base.join(workspace_segment))
        .nfc()
        .collect::<String>()
        .to_ascii_lowercase()
}
