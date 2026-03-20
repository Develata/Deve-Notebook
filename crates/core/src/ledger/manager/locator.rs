use anyhow::{Result, anyhow};

use crate::ledger::listing::RepoListing;
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;

#[derive(Default)]
struct LocalRepoCandidates {
    by_id: Option<String>,
    by_name: Option<String>,
}

impl RepoManager {
    pub fn find_local_repo_name_by_id(&self, target_id: RepoId) -> Result<Option<String>> {
        self.refresh_local_repo_catalog()?;
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

        let local_dir = self.ledger_dir.join("local");
        if !local_dir.exists() {
            anyhow::bail!(
                "Broken local repo catalog: local repo directory missing at {:?}",
                local_dir
            );
        }

        for entry in std::fs::read_dir(local_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("redb") {
                continue;
            }
            let file_stem =
                Self::repo_stem_from_path(&path, "resolving local repo UUID without repair")?;
            if file_stem == self.local_repo_name {
                continue;
            }
            let repo_uuid = Self::read_repo_info_from_path(&path)
                .map_err(|err| {
                    anyhow!(
                        "Broken local repo {} while resolving UUID {} without repair: {}",
                        file_stem,
                        target_id,
                        err
                    )
                })?
                .map(|info| info.uuid);
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
                self.get_repo_info_for(None, Some(normalized))?
                    .ok_or_else(|| {
                        anyhow::anyhow!("Local repo not found for name {}", normalized)
                    })?;
                Some(
                    self.resolve_local_repo_stem(normalized)?
                        .unwrap_or_else(|| normalized.to_string()),
                )
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
        if let Some(name) = candidates
            .by_id
            .clone()
            .or_else(|| candidates.by_name.clone())
        {
            return Ok(name);
        }
        match self.list_repos(None)?.as_slice() {
            [repo] => Ok(repo.clone()),
            [] => anyhow::bail!("No local repositories available"),
            _ => anyhow::bail!("Active repository not selected: multiple local repos exist"),
        }
    }

    fn select_local_repo_name_for_execution(
        &self,
        candidates: &LocalRepoCandidates,
    ) -> Result<String> {
        if let (Some(from_id), Some(from_name)) = (&candidates.by_id, &candidates.by_name)
            && from_id != from_name
        {
            anyhow::bail!(
                "Repo selector mismatch: repo_id resolved to {}, repo_name resolved to {}",
                from_id,
                from_name
            );
        }
        if let Some(name) = candidates
            .by_id
            .clone()
            .or_else(|| candidates.by_name.clone())
        {
            return Ok(name);
        }
        match self.list_local_repo_names_for_execution()?.as_slice() {
            [repo] => Ok(repo.clone()),
            [] => anyhow::bail!("No local repositories available"),
            _ => anyhow::bail!("Active repository not selected: multiple local repos exist"),
        }
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
                let repo_id_label = repo_id
                    .map(|repo_id| repo_id.to_string())
                    .unwrap_or_else(|| "<missing>".to_string());
                tracing::warn!(
                    "UUID-first local repo resolution ignored stale repo_name: repo_id={}, stale_name={}, resolved_name={}",
                    repo_id_label,
                    from_name,
                    from_id
                );
            }
            return Ok(from_id);
        }
        self.select_local_repo_name_for_execution(&candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::RepoManager;
    use crate::ledger::{REPO_METADATA, RepoInfo};
    use tempfile::TempDir;

    fn write_info(db: &redb::Database, info: &RepoInfo) {
        let txn = db.begin_write().expect("write txn");
        txn.open_table(REPO_METADATA)
            .expect("repo metadata")
            .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
            .expect("write metadata");
        txn.commit().expect("commit");
    }

    #[test]
    fn local_repo_id_lookup_without_repair_uses_current_on_disk_metadata() {
        let dir = TempDir::new().expect("tempdir");
        let ledger_dir = dir.path().join("ledger");
        let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
        let wiki = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
        let wiki_info = wiki.get_repo_info().expect("wiki info").expect("present");
        let wiki_db = main.open_database(None, "wiki").expect("wiki db");
        write_info(
            wiki_db.db.as_ref(),
            &RepoInfo {
                uuid: uuid::Uuid::new_v4(),
                name: "wiki".into(),
                url: Some("urn:wiki".into()),
            },
        );

        assert_eq!(
            main.find_local_repo_name_by_id_without_repair(wiki_info.uuid)
                .expect("lookup without repair"),
            None
        );
    }
}
