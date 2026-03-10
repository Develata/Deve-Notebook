use anyhow::Result;
use redb::Database;

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;

impl RepoManager {
    pub fn get_repo_url(&self, branch: Option<&PeerId>, repo_name: &str) -> Result<Option<String>> {
        Ok(self
            .get_repo_info_for(branch, Some(repo_name))?
            .and_then(|info| info.url))
    }

    pub fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        if let Ok(Some(info)) = Self::read_repo_info_from_db(&self.local_db)
            && info.url.as_deref() == Some(target_url)
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
                    let info = Self::read_repo_info_from_db(db)?;
                    Ok(info.and_then(|i| i.url).as_deref() == Some(target_url))
                })
                .unwrap_or(false);
            if is_match {
                return Ok(Some(file_stem.to_string()));
            }
        }

        Ok(None)
    }

    pub fn get_repo_info_for(
        &self,
        branch: Option<&PeerId>,
        repo_name: Option<&str>,
    ) -> Result<Option<RepoInfo>> {
        let name = repo_name
            .unwrap_or(&self.local_repo_name)
            .trim_end_matches(".redb");
        if let Some(peer_id) = branch {
            return self.read_remote_repo_info(peer_id, name);
        }
        if name == self.local_repo_name {
            return Self::read_repo_info_from_db(&self.local_db);
        }
        self.run_on_local_repo(name, Self::read_repo_info_from_db)
    }

    fn read_remote_repo_info(&self, peer_id: &PeerId, repo_name: &str) -> Result<Option<RepoInfo>> {
        if let Ok(repo_id) = uuid::Uuid::parse_str(repo_name) {
            let info = self.run_on_shadow_repo(peer_id, &repo_id, Self::read_repo_info_from_db)?;
            if info.is_some() {
                return Ok(info);
            }
            return Ok(Some(RepoInfo {
                uuid: repo_id,
                name: repo_name.to_string(),
                url: None,
            }));
        }

        let db_path = self
            .remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", repo_name));
        if !db_path.exists() {
            return Ok(None);
        }
        let db = Database::create(&db_path)?;
        Self::read_repo_info_from_db(&db)
    }
}
