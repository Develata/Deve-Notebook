//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-health-and-repair

use crate::ledger::manager::projection_locator::{repo_workspace_segment, safe_repo_path_segment};
use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::{LocalRepoSummary, RepoManager};
use crate::models::RepoId;
use crate::utils::notegit;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const REMOVED_REPOS_FILE: &str = "removed-local-repos.toml";
const REMOVED_REPOS_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RemovedLocalReposFile {
    version: u32,
    #[serde(default)]
    repo_ids: Vec<RepoId>,
}

impl RepoManager {
    pub fn list_local_repo_summaries(&self) -> Result<Vec<LocalRepoSummary>> {
        self.refresh_local_repo_catalog()?;
        let removed = self.removed_local_repo_ids()?;
        let local_dir =
            Self::checked_local_dir_for(&self.ledger_dir, "listing local repo summaries")?;
        let mut summaries = Vec::new();

        for (path, execution_name) in redb_repo_entries(&local_dir, "listing local repo summaries")?
        {
            let info = if execution_name == self.local_repo_name {
                Self::read_repo_info_from_db(&self.local_db)?
            } else {
                Self::read_repo_info_from_path(&path)?
            }
            .ok_or_else(|| {
                anyhow!(
                    "Broken local repo {} while listing local repo summaries: repository metadata missing",
                    execution_name
                )
            })?;

            if removed.contains(&info.uuid) {
                continue;
            }
            summaries.push(LocalRepoSummary {
                repo_id: info.uuid,
                name: info.name,
                execution_name,
            });
        }

        summaries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(summaries)
    }

    pub fn rename_local_repo(&self, repo_id: RepoId, new_name: &str) -> Result<LocalRepoSummary> {
        let new_name = safe_repo_path_segment(new_name.trim())?;
        let stem = self
            .repo_scope_runtime()
            .find_local_repo_name_by_id(repo_id)?
            .ok_or_else(|| anyhow!("Local repo not found for UUID {}", repo_id))?;
        let mut info = self
            .get_repo_info_for(None, Some(&stem))?
            .ok_or_else(|| anyhow!("Local repo metadata is missing for {}", stem))?;
        if info.uuid != repo_id {
            return Err(anyhow!(
                "Local repo {} resolved to {}, expected {}",
                stem,
                info.uuid,
                repo_id
            ));
        }
        if info.name == new_name {
            return Ok(LocalRepoSummary {
                repo_id,
                name: info.name,
                execution_name: stem,
            });
        }
        self.ensure_local_repo_name_available(repo_id, &new_name)?;

        let locator = self.projection_locator_for_repo_id(repo_id)?;
        let old_workspace = locator
            .projection_base_abs
            .join(repo_workspace_segment(&info.name, repo_id)?);
        let new_workspace = locator
            .projection_base_abs
            .join(repo_workspace_segment(&new_name, repo_id)?);
        realign_workspace_root(&old_workspace, &new_workspace, repo_id)?;

        info.name = new_name.clone();
        self.run_on_local_repo_stem(&stem, |db| Self::write_repo_info_to_db(db, &info))?;
        self.set_projection_base_for_repo_id(repo_id, &new_name, locator.projection_base_abs)?;
        notegit::ensure_repo_identity_marker(&new_workspace, repo_id, &new_name)?;
        notegit::ensure_gitignore_ignores_notegit(&new_workspace)
            .with_context(|| format!("Failed to protect .notegit in {:?}", new_workspace))?;

        Ok(LocalRepoSummary {
            repo_id,
            name: new_name,
            execution_name: stem,
        })
    }

    pub fn remove_local_repo(&self, repo_id: RepoId) -> Result<LocalRepoSummary> {
        let summaries = self.list_local_repo_summaries()?;
        if summaries.len() <= 1 {
            return Err(anyhow!("Cannot remove the last local repository"));
        }
        let summary = summaries
            .into_iter()
            .find(|summary| summary.repo_id == repo_id)
            .ok_or_else(|| anyhow!("Local repo not found for UUID {}", repo_id))?;

        self.mark_local_repo_removed(repo_id)?;
        self.remove_projection_locator_for_repo_id(repo_id)?;
        Ok(summary)
    }

    pub(crate) fn is_local_repo_removed(&self, repo_id: RepoId) -> Result<bool> {
        Ok(self.removed_local_repo_ids()?.contains(&repo_id))
    }

    fn ensure_local_repo_name_available(&self, repo_id: RepoId, name: &str) -> Result<()> {
        for summary in self.list_local_repo_summaries()? {
            if summary.repo_id == repo_id {
                continue;
            }
            if summary.name == name || summary.execution_name == name {
                return Err(anyhow!("Repository already exists: {}", name));
            }
        }
        Ok(())
    }

    fn mark_local_repo_removed(&self, repo_id: RepoId) -> Result<()> {
        let mut file = read_removed_local_repos_file(&self.removed_local_repos_path())?;
        if !file.repo_ids.contains(&repo_id) {
            file.repo_ids.push(repo_id);
            file.repo_ids.sort();
        }
        write_removed_local_repos_file(&self.removed_local_repos_path(), &file)
    }

    fn removed_local_repo_ids(&self) -> Result<HashSet<RepoId>> {
        Ok(
            read_removed_local_repos_file(&self.removed_local_repos_path())?
                .repo_ids
                .into_iter()
                .collect(),
        )
    }

    fn removed_local_repos_path(&self) -> PathBuf {
        notegit::host_dir(&self.ledger_dir).join(REMOVED_REPOS_FILE)
    }
}

fn realign_workspace_root(old_root: &PathBuf, new_root: &PathBuf, repo_id: RepoId) -> Result<()> {
    if old_root == new_root {
        notegit::validate_repo_identity_marker(old_root, repo_id)?;
        return Ok(());
    }

    let old_exists = old_root
        .try_exists()
        .with_context(|| format!("Failed to stat old Projection workspace: {:?}", old_root))?;
    let new_exists = new_root
        .try_exists()
        .with_context(|| format!("Failed to stat new Projection workspace: {:?}", new_root))?;

    if new_exists {
        notegit::validate_repo_identity_marker(new_root, repo_id)?;
        if old_exists {
            return Err(anyhow!(
                "Target Projection workspace already exists during repo rename: {:?}",
                new_root
            ));
        }
        return Ok(());
    }
    if !old_exists {
        return Err(anyhow!(
            "Source Projection workspace missing during repo rename: {:?}",
            old_root
        ));
    }
    notegit::validate_repo_identity_marker(old_root, repo_id)?;
    std::fs::rename(old_root, new_root).with_context(|| {
        format!(
            "Failed to move Projection workspace from {:?} to {:?}",
            old_root, new_root
        )
    })
}

fn read_removed_local_repos_file(path: &PathBuf) -> Result<RemovedLocalReposFile> {
    if !path
        .try_exists()
        .with_context(|| format!("Failed to stat removed repo registry: {:?}", path))?
    {
        return Ok(RemovedLocalReposFile {
            version: REMOVED_REPOS_VERSION,
            repo_ids: Vec::new(),
        });
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read removed repo registry: {:?}", path))?;
    let file: RemovedLocalReposFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse removed repo registry: {:?}", path))?;
    if file.version != REMOVED_REPOS_VERSION {
        return Err(anyhow!(
            "Unsupported removed repo registry version {} in {:?}",
            file.version,
            path
        ));
    }
    Ok(file)
}

fn write_removed_local_repos_file(path: &PathBuf, file: &RemovedLocalReposFile) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Err(anyhow!(
            "Removed repo registry path has no parent: {:?}",
            path
        ));
    };
    std::fs::create_dir_all(parent).with_context(|| {
        format!(
            "Failed to create removed repo registry parent: {:?}",
            parent
        )
    })?;
    let content =
        toml::to_string_pretty(file).context("Failed to serialize removed repo registry")?;
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write removed repo registry: {:?}", path))
}

#[cfg(test)]
mod tests {
    use crate::ledger::RepoManager;

    #[test]
    fn rename_local_repo_realigns_workspace_without_changing_repo_id() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let notes = dir.path().join("notes");
        let mut repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
        RepoManager::init(&ledger, 8, Some("research"), Some("urn:research"))?;
        repo.set_projection_base_for_all_local_repos_checked(&notes)?;
        let sync = crate::sync::SyncManager::new_checked(std::sync::Arc::new(repo))?;
        sync.materialize_local_repo("default")?;
        sync.materialize_local_repo("research")?;

        let repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
        let before = repo
            .list_local_repo_summaries()?
            .into_iter()
            .find(|summary| summary.name == "research")
            .expect("research summary");
        let renamed = repo.rename_local_repo(before.repo_id, "lab")?;

        assert_eq!(renamed.repo_id, before.repo_id);
        assert_eq!(renamed.name, "lab");
        let after = repo.list_local_repo_summaries()?;
        assert!(after.iter().any(|summary| summary.name == "lab"));
        assert!(!after.iter().any(|summary| summary.name == "research"));
        let workspace = repo.check_projection_locator_for_local_repo("lab")?;
        assert!(
            workspace
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("lab--"))
        );
        Ok(())
    }

    #[test]
    fn remove_local_repo_hides_it_without_deleting_authority_file() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let notes = dir.path().join("notes");
        let mut repo = RepoManager::init(&ledger, 8, Some("default"), Some("urn:default"))?;
        RepoManager::init(&ledger, 8, Some("research"), Some("urn:research"))?;
        repo.set_projection_base_for_all_local_repos_checked(&notes)?;

        let summary = repo
            .list_local_repo_summaries()?
            .into_iter()
            .find(|summary| summary.name == "research")
            .expect("research summary");
        let authority = ledger
            .join("local")
            .join(format!("{}.redb", summary.execution_name));

        repo.remove_local_repo(summary.repo_id)?;

        assert!(authority.is_file());
        let summaries = repo.list_local_repo_summaries()?;
        assert!(!summaries.iter().any(|item| item.repo_id == summary.repo_id));
        assert!(repo.get_local_repo_info_by_id(summary.repo_id)?.is_none());
        Ok(())
    }
}
