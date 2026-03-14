use anyhow::Result;

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;

impl RepoManager {
    pub fn get_local_repo_info_by_id(&self, repo_id: uuid::Uuid) -> Result<Option<RepoInfo>> {
        self.repair_local_repo_catalog()?;
        self.get_local_repo_info_by_id_without_repair(repo_id)
    }

    pub(crate) fn get_local_repo_info_by_id_without_repair(
        &self,
        repo_id: uuid::Uuid,
    ) -> Result<Option<RepoInfo>> {
        let Some(repo_name) = self.find_local_repo_name_by_id_without_repair(repo_id)? else {
            return Ok(None);
        };
        if repo_name == self.local_repo_name {
            return Self::read_repo_info_from_db(&self.local_db);
        }
        self.run_on_local_repo(&repo_name, Self::read_repo_info_from_db)
    }

    pub fn get_repo_url(&self, branch: Option<&PeerId>, repo_name: &str) -> Result<Option<String>> {
        Ok(self
            .get_repo_info_for(branch, Some(repo_name))?
            .and_then(|info| info.url))
    }

    pub fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        self.repair_local_repo_catalog()?;
        let matches = self
            .list_local_repo_names_for_execution()?
            .into_iter()
            .filter_map(|repo_name| {
                self.get_repo_info_for(None, Some(&repo_name))
                    .ok()
                    .flatten()
                    .filter(|info| info.url.as_deref() == Some(target_url))
                    .map(|_| repo_name)
            })
            .collect::<Vec<_>>();
        Ok((matches.len() == 1).then(|| matches[0].clone()))
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
        if let Some(stem) = self.resolve_local_repo_stem(name)? {
            return if stem == self.local_repo_name {
                Self::read_repo_info_from_db(&self.local_db)
            } else {
                self.run_on_local_repo_stem(&stem, Self::read_repo_info_from_db)
            };
        }
        self.repair_local_repo_catalog()?;
        if let Some(stem) = self.resolve_local_repo_stem(name)? {
            return if stem == self.local_repo_name {
                Self::read_repo_info_from_db(&self.local_db)
            } else {
                self.run_on_local_repo_stem(&stem, Self::read_repo_info_from_db)
            };
        }
        Ok(None)
    }

    fn read_remote_repo_info(&self, peer_id: &PeerId, repo_name: &str) -> Result<Option<RepoInfo>> {
        if let Some(entry) = self.resolve_remote_repo_entry(peer_id, repo_name)? {
            if let Some(info) = entry.info {
                return Ok(Some(info));
            }
            if let Some(info) = Self::read_repo_info_from_path(&entry.path)? {
                return Ok(Some(info));
            }
            if let Ok(repo_id) = uuid::Uuid::parse_str(&entry.stem) {
                return Ok(Some(RepoInfo {
                    uuid: repo_id,
                    name: entry.stem,
                    url: None,
                }));
            }
            return Ok(None);
        }
        Ok(None)
    }
}
