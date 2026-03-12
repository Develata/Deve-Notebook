use anyhow::Result;

use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;

#[derive(Default)]
struct LocalRepoCandidates {
    by_id: Option<String>,
    by_name: Option<String>,
}

impl RepoManager {
    pub fn find_local_repo_name_by_id(&self, target_id: RepoId) -> Result<Option<String>> {
        if let Ok(Some(info)) = Self::read_repo_info_from_db(&self.local_db)
            && info.uuid == target_id
        {
            return Ok(Some(self.local_repo_name.clone()));
        }

        let local_dir = self.ledger_dir.join("local");
        if !local_dir.exists() {
            return Ok(None);
        }

        for entry in std::fs::read_dir(local_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("redb") {
                continue;
            }
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if file_stem == self.local_repo_name {
                continue;
            }
            let is_match = self
                .run_on_local_repo(file_stem, |db| {
                    Ok(Self::read_repo_info_from_db(db)?.map(|info| info.uuid) == Some(target_id))
                })
                .unwrap_or(false);
            if is_match {
                return Ok(Some(file_stem.to_string()));
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
                self.get_repo_info_for(None, Some(normalized))?
                    .ok_or_else(|| {
                        anyhow::anyhow!("Local repo not found for name {}", normalized)
                    })?;
                Some(normalized.to_string())
            }
            None => None,
        };
        Ok(LocalRepoCandidates { by_id, by_name })
    }

    fn select_local_repo_name(&self, candidates: &LocalRepoCandidates) -> Result<String> {
        if let (Some(from_id), Some(from_name)) = (&candidates.by_id, &candidates.by_name)
            && from_id != from_name
        {
            anyhow::bail!(
                "Repo selector mismatch: repo_id resolved to {}, repo_name resolved to {}",
                from_id,
                from_name
            );
        }
        Ok(candidates
            .by_id
            .clone()
            .or_else(|| candidates.by_name.clone())
            .unwrap_or_else(|| self.local_repo_name.clone()))
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
        self.repair_local_repo_catalog()?;
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
    /// - 执行级本地 repo 解析在已拿到 `RepoUUID` 时必须优先采用 UUID。
    /// - `repo_name` 仅作为缺失 UUID 时的回退与诊断信息，不得反向覆盖已解析 UUID。
    pub fn resolve_local_repo_name_for_execution(
        &self,
        repo_id: Option<RepoId>,
        repo_name: Option<&str>,
    ) -> Result<String> {
        let candidates = self.resolve_local_repo_candidates_with_repair(repo_id, repo_name)?;
        if let Some(from_id) = candidates.by_id {
            if let Some(from_name) = candidates.by_name
                && from_name != from_id
            {
                tracing::warn!(
                    "UUID-first local repo resolution ignored stale repo_name: repo_id={}, stale_name={}, resolved_name={}",
                    repo_id.expect("from_id requires repo_id"),
                    from_name,
                    from_id
                );
            }
            return Ok(from_id);
        }
        self.select_local_repo_name(&candidates)
    }
}
