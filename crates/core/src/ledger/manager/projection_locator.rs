//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract
//!   - 04_repository#repo-health-and-repair

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::RepoId;
use crate::utils::notegit;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

const LOCATOR_VERSION: u32 = 1;
const LOCATOR_FILE: &str = "projection-locators.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionLocatorRecord {
    pub repo_id: RepoId,
    pub repo_name_hint: String,
    pub projection_base_abs: PathBuf,
    pub canonicalized_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProjectionLocatorFile {
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) locators: Vec<ProjectionLocatorRecord>,
}

impl Default for ProjectionLocatorFile {
    fn default() -> Self {
        Self {
            version: LOCATOR_VERSION,
            locators: Vec::new(),
        }
    }
}

impl RepoManager {
    pub fn projection_locator_path(&self) -> PathBuf {
        projection_locator_path_for(&self.ledger_dir)
    }

    pub fn set_projection_base_for_local_repo(
        &self,
        repo_name: &str,
        projection_base: impl AsRef<Path>,
    ) -> Result<ProjectionLocatorRecord> {
        let info = self.local_repo_info_for_locator(repo_name)?;
        self.set_projection_base_for_repo_id(info.uuid, &info.name, projection_base)
    }

    /// 为所有当前本地 repo 设置同一个 Projection Locator base。
    ///
    /// Invariants:
    /// - 参数是 projection base；最终 workspace root 为 `<base>/<safe_repo_name>--<repo_id>/`。
    /// - 批量更新必须按最终 locator map 校验并一次写入，不能暴露或误判中间混合态。
    /// - 生产入口应优先通过 `set_projection_base_for_local_repo` 明确绑定目标 repo。
    pub fn set_projection_base_for_all_local_repos_checked(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<()> {
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
        file.locators
            .retain(|record| !local_ids.contains(&record.repo_id));
        for info in local_infos {
            file.locators.push(ProjectionLocatorRecord {
                repo_id: info.uuid,
                repo_name_hint: safe_repo_path_segment(&info.name)?,
                projection_base_abs: projection_base_abs.clone(),
                canonicalized_at_unix_ms: timestamp,
            });
        }
        file.locators
            .sort_by_key(|item| (item.repo_name_hint.clone(), item.repo_id));
        self.validate_projection_locator_records(&file.locators, true)?;
        self.write_projection_locator_file(&file)
    }

    pub fn set_projection_base_for_repo_id(
        &self,
        repo_id: RepoId,
        repo_name_hint: &str,
        projection_base: impl AsRef<Path>,
    ) -> Result<ProjectionLocatorRecord> {
        let repo_name_hint = safe_repo_path_segment(repo_name_hint)?;
        let projection_base_abs = canonicalize_projection_base(projection_base.as_ref())?;
        let record = ProjectionLocatorRecord {
            repo_id,
            repo_name_hint,
            projection_base_abs,
            canonicalized_at_unix_ms: chrono::Utc::now().timestamp_millis(),
        };
        let mut file = self.read_projection_locator_file()?;
        file.locators.retain(|item| item.repo_id != repo_id);
        file.locators.push(record.clone());
        file.locators
            .sort_by_key(|item| (item.repo_name_hint.clone(), item.repo_id));
        self.validate_projection_locator_records(&file.locators, false)?;
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

    pub fn validated_projection_locator_for_repo_id(
        &self,
        repo_id: RepoId,
    ) -> Result<ProjectionLocatorRecord> {
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
        for locator in &mut locators {
            locator.projection_base_abs = std::fs::canonicalize(&locator.projection_base_abs)
                .with_context(|| {
                    format!(
                        "Failed to canonicalize Projection Locator base for repo {}: {:?}",
                        locator.repo_id, locator.projection_base_abs
                    )
                })?;
        }
        locators.sort_by_key(|item| (item.repo_name_hint.clone(), item.repo_id));
        Ok(locators)
    }

    pub fn remove_projection_locator_for_repo_id(&self, repo_id: RepoId) -> Result<()> {
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
        let workspace_root = locator
            .projection_base_abs
            .join(repo_workspace_segment(&info.name, info.uuid)?);
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
        validate_projection_locator_records(self, records, require_all_local_locators)
    }
}

pub(crate) fn projection_locator_path_for(ledger_dir: &Path) -> PathBuf {
    notegit::host_dir(ledger_dir).join(LOCATOR_FILE)
}

pub(crate) fn read_projection_locator_file(path: &Path) -> Result<ProjectionLocatorFile> {
    if !path
        .try_exists()
        .with_context(|| format!("Failed to stat Projection Locator file: {:?}", path))?
    {
        return Ok(ProjectionLocatorFile::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Projection Locator file: {:?}", path))?;
    let file: ProjectionLocatorFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse Projection Locator file: {:?}", path))?;
    if file.version != LOCATOR_VERSION {
        return Err(anyhow!(
            "Unsupported Projection Locator version {} in {:?}",
            file.version,
            path
        ));
    }
    Ok(file)
}

fn write_projection_locator_file(path: &Path, file: &ProjectionLocatorFile) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Err(anyhow!("Projection Locator path has no parent: {:?}", path));
    };
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create Projection Locator parent: {:?}", parent))?;
    let content = toml::to_string_pretty(file).context("Failed to serialize Projection Locator")?;
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write Projection Locator file: {:?}", path))
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

pub(crate) fn repo_workspace_segment(repo_name: &str, repo_id: RepoId) -> Result<String> {
    Ok(format!(
        "{}--{}",
        safe_repo_path_segment(repo_name)?,
        repo_id
    ))
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

fn validate_projection_locator_records(
    repo: &RepoManager,
    records: &[ProjectionLocatorRecord],
    require_all_local_locators: bool,
) -> Result<()> {
    let mut records_by_id = HashMap::new();
    for record in records {
        if records_by_id.insert(record.repo_id, record).is_some() {
            return Err(anyhow!(
                "Projection Locator contains duplicate record for repo {}",
                record.repo_id
            ));
        }
    }

    let mut local_infos_by_id = HashMap::new();
    let mut local_infos = Vec::new();
    for repo_name in repo.list_local_repo_names_for_execution()? {
        let info = repo
            .get_repo_info_for(None, Some(&repo_name))?
            .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", repo_name))?;
        local_infos_by_id.insert(info.uuid, repo_name.clone());
        local_infos.push((repo_name, info));
    }

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
    let mut roots: Vec<(RepoId, PathBuf, String)> = Vec::new();
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
        if !record.projection_base_abs.is_absolute() {
            return Err(anyhow!(
                "Projection Locator for {} must use an absolute projection base",
                repo_name
            ));
        }
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
        roots.push((
            info.uuid,
            root,
            normalized_workspace_key(&projection_base_abs, &workspace_segment),
        ));
    }

    for idx in 0..roots.len() {
        for other in (idx + 1)..roots.len() {
            let (left_id, left_root, left_key) = &roots[idx];
            let (right_id, right_root, right_key) = &roots[other];
            if left_key == right_key {
                return Err(anyhow!(
                    "Projection workspace conflict: repos {} and {} resolve to {:?}",
                    left_id,
                    right_id,
                    left_root
                ));
            }
            if left_root.starts_with(right_root) || right_root.starts_with(left_root) {
                return Err(anyhow!(
                    "Projection workspace nesting conflict between {:?} and {:?}",
                    left_root,
                    right_root
                ));
            }
        }
    }
    Ok(())
}

fn normalized_workspace_key(base: &Path, workspace_segment: &str) -> String {
    crate::utils::path::path_to_forward_slash(&base.join(workspace_segment))
        .nfc()
        .collect::<String>()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests;
