//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract
//!   - 04_repository#repo-catalog-contract

use super::{ProjectionLocatorRecord, file_validation, repo_workspace_segment};
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::RepoId;
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
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

pub(super) fn validate_projection_locator_records_for_workspace_repair(
    repo: &RepoManager,
    records: &[ProjectionLocatorRecord],
    repo_id: RepoId,
    proposed_name: &str,
    old_root: &Path,
    new_root: &Path,
) -> Result<()> {
    let proposed_name = super::safe_repo_path_segment(proposed_name)?;
    let mut local_infos = repo
        .repo_catalog_runtime()
        .local_repo_infos_for_locator_repair_validation()?;
    let target = local_infos
        .iter_mut()
        .find(|(_, info)| info.uuid == repo_id)
        .ok_or_else(|| {
            anyhow!(
                "Projection Locator repair target repo {} is unknown",
                repo_id
            )
        })?;
    target.1.name = proposed_name;
    let roots = resolved_roots_against_local_infos(repo, records, true, local_infos)?;
    validate_workspace_repair_roots(&roots, repo_id, old_root, new_root)
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
        .map(|(stem, info)| (info.uuid, stem))
        .collect::<HashMap<_, _>>();

    for record in records {
        if !local_infos_by_id.contains_key(&record.repo_id) {
            return Err(anyhow!(
                "Projection Locator references unknown local repo {}",
                record.repo_id
            ));
        }
    }

    let ledger_dir =
        std::fs::canonicalize(&repo.ledger_dir).unwrap_or_else(|_| repo.ledger_dir.clone());
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
        let projection_base_abs =
            std::fs::canonicalize(&record.projection_base_abs).with_context(|| {
                format!(
                    "Failed to canonicalize Projection Locator base for {}: {:?}",
                    repo_name, record.projection_base_abs
                )
            })?;
        let workspace_segment = repo_workspace_segment(&info.name, info.uuid)?;
        let root = projection_base_abs.join(&workspace_segment);
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
        roots.push(ResolvedRoot {
            repo_id: info.uuid,
            path: root,
            normalized_key: normalized_workspace_key(&projection_base_abs, &workspace_segment),
        });
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

fn validate_workspace_repair_roots(
    roots: &[ResolvedRoot],
    repo_id: RepoId,
    old_root: &Path,
    new_root: &Path,
) -> Result<()> {
    let target = roots
        .iter()
        .find(|root| root.repo_id == repo_id)
        .ok_or_else(|| {
            anyhow!(
                "Projection Locator repair target repo {} is missing",
                repo_id
            )
        })?;
    let canonical_old_root = std::fs::canonicalize(old_root).with_context(|| {
        format!(
            "Failed to canonicalize Projection Locator repair source: {:?}",
            old_root
        )
    })?;
    let expected_new_key = crate::utils::path::path_to_forward_slash(new_root)
        .nfc()
        .collect::<String>()
        .to_ascii_lowercase();
    if target.normalized_key != expected_new_key {
        return Err(anyhow!(
            "Projection Locator repair target mismatch: expected {:?}, resolved {:?}",
            new_root,
            target.path
        ));
    }
    for other in roots.iter().filter(|root| root.repo_id != repo_id) {
        if canonical_old_root.starts_with(&other.path)
            || other.path.starts_with(&canonical_old_root)
        {
            return Err(anyhow!(
                "Projection workspace nesting conflict between repair source {:?} and repo {} root {:?}",
                canonical_old_root,
                other.repo_id,
                other.path
            ));
        }
    }
    Ok(())
}

pub(super) fn normalized_workspace_key(base: &Path, workspace_segment: &str) -> String {
    crate::utils::path::path_to_forward_slash(&base.join(workspace_segment))
        .nfc()
        .collect::<String>()
        .to_ascii_lowercase()
}
