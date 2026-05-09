//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract

use anyhow::{Result, anyhow};

use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::repo_selector_resolution::{LocalRepoCandidates, select_repo_name};
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;

impl RepoManager {
    pub fn find_local_repo_name_by_id(&self, target_id: RepoId) -> Result<Option<String>> {
        self.repo_catalog_runtime().refresh_local_catalog()?;
        self.find_local_repo_name_by_id_without_repair(target_id)
    }

    pub(crate) fn find_local_repo_name_by_id_without_repair(
        &self,
        target_id: RepoId,
    ) -> Result<Option<String>> {
        if let Some(info) = Self::read_repo_info_from_db(&self.local_db).map_err(|err| {
            anyhow!(
                "Broken local repo {} while resolving UUID {} without repair: {}",
                self.local_repo_name,
                target_id,
                err
            )
        })? && info.uuid == target_id
        {
            return Ok(Some(self.local_repo_name.clone()));
        }

        let local_dir = Self::checked_local_dir_for(
            &self.ledger_dir,
            "resolving local repo UUID without repair",
        )?;

        for (path, file_stem) in
            redb_repo_entries(&local_dir, "resolving local repo UUID without repair")?
        {
            if file_stem == self.local_repo_name {
                continue;
            }
            let repo_uuid = Some(
                Self::read_required_repo_info_from_path(
                    &path,
                    &file_stem,
                    "resolving UUID without repair",
                )
                .map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while resolving UUID {} without repair: {}",
                        file_stem,
                        target_id,
                        err
                    )
                })?
                .uuid,
            );
            if repo_uuid == Some(target_id) {
                return Ok(Some(file_stem));
            }
        }

        Ok(None)
    }

    fn resolve_local_repo_candidates(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<LocalRepoCandidates> {
        let by_id = match repo_id {
            Some(repo_id) => Some(
                self.find_local_repo_name_by_id(repo_id)?
                    .ok_or_else(|| anyhow::anyhow!("Local repo not found for UUID {}", repo_id))?,
            ),
            None => None,
        };
        let by_name = match repo_name {
            Some(repo_name) => {
                let normalized = repo_name.trim_end_matches(".redb");
                if let Some(stem) = self.resolve_local_repo_stem(normalized)? {
                    self.get_repo_info_for(None, Some(&stem))?
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Broken local repo {} while resolving selector {}: repository metadata missing",
                                stem,
                                normalized
                            )
                        })?;
                    Some(stem)
                } else {
                    anyhow::bail!("Local repo not found for name {}", normalized);
                }
            }
            None => None,
        };
        Ok(LocalRepoCandidates { by_id, by_name })
    }

    fn select_local_repo_name(&self, candidates: &LocalRepoCandidates) -> Result<String> {
        select_repo_name(candidates, || {
            self.repo_catalog_runtime().list_local_display_names()
        })
    }

    fn select_local_repo_name_for_execution(
        &self,
        candidates: &LocalRepoCandidates,
    ) -> Result<String> {
        select_repo_name(candidates, || {
            self.repo_catalog_runtime().list_local_execution_names()
        })
    }

    fn resolve_local_repo_candidates_with_repair(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<LocalRepoCandidates> {
        let initial = self.resolve_local_repo_candidates(repo_id, repo_name)?;
        if self.select_local_repo_name(&initial).is_ok() {
            return Ok(initial);
        }
        self.refresh_local_repo_catalog()?;
        self.resolve_local_repo_candidates(repo_id, repo_name)
    }

    /// Invariant: 进入本地 DB 写路径前，repo selector 必须被解析为单一 repo 名称。
    pub fn resolve_local_repo_name(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        let initial = self.resolve_local_repo_candidates_with_repair(repo_id, repo_name)?;
        self.select_local_repo_name(&initial)
    }

    /// Invariants:
    /// - 执行级本地 repo 解析必须保证 `RepoUUID` 与 `repo_name` 指向同一 repo。
    /// - `repo_name` 仅作为缺失 UUID 时的回退与诊断信息，不得覆盖或被 UUID 静默覆盖。
    pub fn resolve_local_repo_name_for_execution(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        let candidates = self.resolve_local_repo_candidates_with_repair(repo_id, repo_name)?;
        self.select_local_repo_name_for_execution(&candidates)
    }
}

#[cfg(test)]
mod tests;
