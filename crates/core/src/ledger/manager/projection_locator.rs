//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract
//!   - 04_repository#repo-health-and-repair

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::RepoId;
use crate::utils::notegit;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

mod activation;
mod file_validation;
mod map_validation;
mod removal;
mod store;

pub use removal::{ProjectionLocatorCleanupDisposition, ProjectionLocatorRemovalPlan};
pub(crate) use store::projection_locator_record_for_repo_id;
use store::{
    ProjectionLocatorFile, ProjectionLocatorMapGuard, projection_locator_path_for,
    read_projection_locator_file, write_projection_locator_file,
};

pub(crate) use activation::ProjectionLocatorActivationGuard;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionLocatorRecord {
    pub repo_id: RepoId,
    pub workspace_segment: String,
    pub projection_base_abs: PathBuf,
    pub canonicalized_at_unix_ms: i64,
}

impl RepoManager {
    fn projection_locator_path(&self) -> PathBuf {
        projection_locator_path_for(&self.ledger_dir)
    }

    pub fn set_projection_base_for_local_repo(
        &self,
        repo_name: &str,
        projection_base: impl AsRef<Path>,
    ) -> Result<ProjectionLocatorRecord> {
        let info = self.local_repo_info_for_locator(repo_name)?;
        self.set_projection_base_for_repo_id(info.uuid, projection_base)
    }

    /// 为所有当前本地 repo 设置同一个 Projection Locator base。
    ///
    /// Invariants:
    /// - 参数是 projection base；最终 workspace root 为 `<base>/<workspace_segment>/`。
    /// - 批量更新必须按最终 locator map 校验并一次写入，不能暴露或误判中间混合态。
    /// - 生产入口应优先通过 `set_projection_base_for_local_repo` 明确绑定目标 repo。
    pub fn set_projection_base_for_all_local_repos_checked(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<()> {
        let _map_guard = ProjectionLocatorMapGuard::acquire(&self.ledger_dir)?;
        self.refresh_local_repo_catalog()?;
        let projection_base_abs = canonicalize_projection_base(root.as_ref())?;
        let timestamp = chrono::Utc::now().timestamp_millis();
        let local_infos = self
            .list_local_repo_names_for_execution()?
            .into_iter()
            .map(|repo_name| {
                self.get_repo_info_for(None, Some(&repo_name))?
                    .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))
            })
            .collect::<Result<Vec<_>>>()?;
        let local_ids = local_infos
            .iter()
            .map(|info| info.uuid)
            .collect::<HashSet<_>>();

        let mut file = self.read_projection_locator_file()?;
        let existing_segments = file
            .locators
            .iter()
            .map(|record| (record.repo_id, record.workspace_segment.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        file.locators
            .retain(|record| !local_ids.contains(&record.repo_id));
        for info in local_infos {
            file.locators.push(ProjectionLocatorRecord {
                repo_id: info.uuid,
                workspace_segment: existing_segments
                    .get(&info.uuid)
                    .cloned()
                    .unwrap_or_else(|| info.uuid.to_string()),
                projection_base_abs: projection_base_abs.clone(),
                canonicalized_at_unix_ms: timestamp,
            });
        }
        file.locators
            .sort_by_key(|item| (item.workspace_segment.clone(), item.repo_id));
        self.validate_projection_locator_records(&file.locators, true)?;
        self.write_projection_locator_file(&file)
    }

    pub fn set_projection_base_for_repo_id(
        &self,
        repo_id: RepoId,
        projection_base: impl AsRef<Path>,
    ) -> Result<ProjectionLocatorRecord> {
        let _map_guard = ProjectionLocatorMapGuard::acquire(&self.ledger_dir)?;
        // Normal locator set requires Normal catalog membership; only the
        // prepared-creation command may bind a not-yet-cataloged RepoId
        // (03_storage/projection: no generic allow-unknown bypass).
        let is_member = self
            .repo_catalog_membership_record(repo_id)?
            .is_some_and(|record| {
                record.state() == crate::ledger::RepoCatalogMembershipState::Normal
            });
        if !is_member {
            return Err(anyhow!(
                "Projection Locator set rejects unknown local repo {repo_id}: normal catalog membership required"
            ));
        }
        let projection_base_abs = canonicalize_projection_base(projection_base.as_ref())?;
        let mut file = self.read_projection_locator_file()?;
        let workspace_segment = file
            .locators
            .iter()
            .find(|item| item.repo_id == repo_id)
            .map(|item| item.workspace_segment.clone())
            .unwrap_or_else(|| repo_id.to_string());
        let record = ProjectionLocatorRecord {
            repo_id,
            workspace_segment,
            projection_base_abs,
            canonicalized_at_unix_ms: chrono::Utc::now().timestamp_millis(),
        };
        file.locators.retain(|item| item.repo_id != repo_id);
        file.locators.push(record.clone());
        file.locators
            .sort_by_key(|item| (item.workspace_segment.clone(), item.repo_id));
        self.validate_projection_locator_records(&file.locators, false)?;
        self.write_projection_locator_file(&file)?;
        Ok(record)
    }

    /// Publishes the locator half of one prepared repo creation without making
    /// the repo visible to normal catalog-backed locator queries.
    pub fn prepare_projection_locator_for_repo_creation(
        &self,
        repo_id: RepoId,
        projection_base: impl AsRef<Path>,
    ) -> Result<ProjectionLocatorRecord> {
        let info = self.with_initial_primary_for_catalog(repo_id, |db| {
            RepoManager::read_local_repo_info_from_db(db)
                .map_err(crate::ledger::LocalAuthorityError::Other)?
                .ok_or_else(|| {
                    crate::ledger::LocalAuthorityError::Invariant(format!(
                        "Prepared Projection Locator target metadata is missing: {repo_id}"
                    ))
                })
        })?;
        self.prepare_projection_locator_for_repo_creation_inner(
            repo_id,
            projection_base.as_ref(),
            info,
        )
    }

    pub fn prepare_projection_locator_for_repo_creation_with_authority(
        &self,
        repo_id: RepoId,
        projection_base: impl AsRef<Path>,
        authority: &crate::ledger::PreparedRepoAuthority,
    ) -> Result<ProjectionLocatorRecord> {
        if authority.repo_id() != repo_id {
            return Err(anyhow!(
                "Prepared Projection Locator authority RepoId mismatch: expected {repo_id}, got {}",
                authority.repo_id()
            ));
        }
        let info = RepoManager::read_local_repo_info_from_db(authority.db())?.ok_or_else(|| {
            anyhow!("Prepared Projection Locator target metadata is missing: {repo_id}")
        })?;
        self.prepare_projection_locator_for_repo_creation_inner(
            repo_id,
            projection_base.as_ref(),
            info,
        )
    }

    fn prepare_projection_locator_for_repo_creation_inner(
        &self,
        repo_id: RepoId,
        projection_base: &Path,
        info: RepoInfo,
    ) -> Result<ProjectionLocatorRecord> {
        let _map_guard = ProjectionLocatorMapGuard::acquire(&self.ledger_dir)?;
        if self.repo_catalog_membership_record(repo_id)?.is_some() {
            return Err(anyhow!(
                "Prepared Projection Locator target already has catalog state: {repo_id}"
            ));
        }
        let execution_name = repo_id.to_string();
        if info.uuid != repo_id || info.name != execution_name {
            return Err(anyhow!(
                "Prepared Projection Locator target must use canonical RepoId identity: expected {repo_id}, metadata uuid={}, name={:?}",
                info.uuid,
                info.name
            ));
        }
        let projection_base_abs = canonicalize_projection_base(projection_base)?;
        let record = ProjectionLocatorRecord {
            repo_id,
            workspace_segment: execution_name,
            projection_base_abs,
            canonicalized_at_unix_ms: chrono::Utc::now().timestamp_millis(),
        };
        let mut file = self.read_projection_locator_file()?;
        if let Some(existing) = file.locators.iter().find(|item| item.repo_id == repo_id)
            && existing.workspace_segment != record.workspace_segment
        {
            return Err(anyhow!(
                "Prepared Projection Locator target has a conflicting immutable workspace segment"
            ));
        }
        file.locators.retain(|item| item.repo_id != repo_id);
        file.locators.push(record.clone());
        file.locators
            .sort_by_key(|item| (item.workspace_segment.clone(), item.repo_id));
        map_validation::validate_projection_locator_records_for_prepared_creation(
            self,
            &file.locators,
            repo_id,
            info,
        )?;
        self.write_projection_locator_file(&file)?;
        Ok(record)
    }

    pub fn projection_locator_for_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<ProjectionLocatorRecord> {
        let info = self.local_repo_info_for_locator(repo_name)?;
        self.projection_locator_for_repo_id(info.uuid)
            .with_context(|| format!("Projection Locator missing for local repo {}", info.name))
    }

    pub fn projection_locator_for_repo_id(
        &self,
        repo_id: RepoId,
    ) -> Result<ProjectionLocatorRecord> {
        self.validated_projection_locator_for_repo_id(repo_id)
    }

    /// Reads the independent locator truth for lifecycle partial-outcome
    /// classification. Missing is represented as `None`; malformed or
    /// non-canonical locator evidence remains a fail-closed error.
    pub fn query_projection_locator_record_for_repo_id(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<ProjectionLocatorRecord>> {
        projection_locator_record_for_repo_id(&self.ledger_dir, repo_id)
    }

    pub fn validated_projection_locator_for_repo_id(
        &self,
        repo_id: RepoId,
    ) -> Result<ProjectionLocatorRecord> {
        if self.find_local_repo_name_by_id(repo_id)?.is_none() {
            return Err(anyhow!(
                "Projection Locator target is not a cataloged local repo: {repo_id}"
            ));
        }
        let file = self.read_projection_locator_file()?;
        self.validate_projection_locator_records(&file.locators, true)?;
        let mut locator = file
            .locators
            .into_iter()
            .find(|item| item.repo_id == repo_id)
            .ok_or_else(|| anyhow!("Projection Locator missing for repo {}", repo_id))?;
        locator.projection_base_abs = std::fs::canonicalize(&locator.projection_base_abs)
            .with_context(|| {
                format!(
                    "Failed to canonicalize Projection Locator base for repo {}: {:?}",
                    repo_id, locator.projection_base_abs
                )
            })?;
        Ok(locator)
    }

    pub fn list_projection_locators(&self) -> Result<Vec<ProjectionLocatorRecord>> {
        let mut locators = self.read_projection_locator_file()?.locators;
        self.validate_projection_locator_records(&locators, false)?;
        let cataloged = self
            .list_cataloged_local_repo_summaries()?
            .into_iter()
            .map(|summary| summary.repo_id)
            .collect::<HashSet<_>>();
        locators.retain(|locator| cataloged.contains(&locator.repo_id));
        for locator in &mut locators {
            locator.projection_base_abs = std::fs::canonicalize(&locator.projection_base_abs)
                .with_context(|| {
                    format!(
                        "Failed to canonicalize Projection Locator base for repo {}: {:?}",
                        locator.repo_id, locator.projection_base_abs
                    )
                })?;
        }
        locators.sort_by_key(|item| (item.workspace_segment.clone(), item.repo_id));
        Ok(locators)
    }

    pub fn remove_projection_locator_for_repo_id(&self, repo_id: RepoId) -> Result<()> {
        let _map_guard = ProjectionLocatorMapGuard::acquire(&self.ledger_dir)?;
        let mut file = self.read_projection_locator_file()?;
        file.locators.retain(|item| item.repo_id != repo_id);
        self.validate_projection_locator_records(&file.locators, false)?;
        self.write_projection_locator_file(&file)
    }

    pub fn validate_projection_locator_map(&self) -> Result<()> {
        let file = self.read_projection_locator_file()?;
        self.validate_projection_locator_records(&file.locators, true)
    }

    pub fn check_projection_locator_for_local_repo(&self, repo_name: &str) -> Result<PathBuf> {
        let info = self.local_repo_info_for_locator(repo_name)?;
        let locator = self.validated_projection_locator_for_repo_id(info.uuid)?;
        let workspace_root = locator.projection_base_abs.join(&locator.workspace_segment);
        let workspace_root = std::fs::canonicalize(&workspace_root).with_context(|| {
            format!(
                "Failed to canonicalize Projection workspace root for local repo {}: {:?}",
                info.name, workspace_root
            )
        })?;
        notegit::validate_repo_identity_marker(&workspace_root, info.uuid)?;
        Ok(workspace_root)
    }

    fn local_repo_info_for_locator(&self, repo_name: &str) -> Result<RepoInfo> {
        let stem = self
            .resolve_local_repo_stem(repo_name)?
            .ok_or_else(|| anyhow!("Repository not found: {}", repo_name))?;
        self.get_repo_info_for(None, Some(&stem))?
            .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", stem))
    }

    fn read_projection_locator_file(&self) -> Result<ProjectionLocatorFile> {
        read_projection_locator_file(&self.projection_locator_path())
    }

    fn write_projection_locator_file(&self, file: &ProjectionLocatorFile) -> Result<()> {
        write_projection_locator_file(&self.projection_locator_path(), file)
    }

    fn validate_projection_locator_records(
        &self,
        records: &[ProjectionLocatorRecord],
        require_all_local_locators: bool,
    ) -> Result<()> {
        map_validation::validate_projection_locator_records(
            self,
            records,
            require_all_local_locators,
        )
    }
}

fn canonicalize_projection_base(base: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(base)
        .with_context(|| format!("Failed to create projection base: {:?}", base))?;
    std::fs::canonicalize(base)
        .with_context(|| format!("Failed to canonicalize projection base: {:?}", base))
}

pub(crate) fn safe_repo_path_segment(repo_name: &str) -> Result<String> {
    let segment = repo_name;
    if segment.is_empty() {
        return Err(anyhow!("repo_name must not be empty"));
    }
    if segment == "." || segment == ".." {
        return Err(anyhow!("repo_name must not be . or .."));
    }
    if segment.ends_with(' ') || segment.ends_with('.') {
        return Err(anyhow!("repo_name must not end with a space or dot"));
    }
    if segment.contains('\0') {
        return Err(anyhow!("repo_name must not contain NUL"));
    }
    if segment
        .chars()
        .any(|ch| matches!(ch, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err(anyhow!(
            "repo_name must be a single safe path segment: {}",
            segment
        ));
    }
    let path = Path::new(segment);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || path.components().count() != 1
    {
        return Err(anyhow!(
            "repo_name must be a single safe path segment: {}",
            segment
        ));
    }
    if is_windows_reserved_device_name(segment) {
        return Err(anyhow!("repo_name uses a reserved Windows device name"));
    }
    Ok(segment.to_string())
}

fn is_windows_reserved_device_name(segment: &str) -> bool {
    let stem = segment
        .split_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(segment)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=9).contains(&n))
        || stem
            .strip_prefix("LPT")
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=9).contains(&n))
}

#[cfg(test)]
use map_validation::normalized_workspace_key;
#[cfg(test)]
mod tests;
