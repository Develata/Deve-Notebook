use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct RemoteRepoEntry {
    pub path: PathBuf,
    pub stem: String,
    pub info: Option<RepoInfo>,
}

impl RepoManager {
    pub(crate) fn scan_remote_repo_entries(
        &self,
        peer_id: &PeerId,
    ) -> Result<Vec<RemoteRepoEntry>> {
        let loaded = self.loaded_remote_repo_info(peer_id);
        let peer_dir = self.remotes_dir().join(peer_id.to_filename());
        if !peer_dir.exists() {
            return Ok(vec![]);
        }
        let mut repos = Vec::new();
        for entry in std::fs::read_dir(peer_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("redb") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let info = match loaded
                .iter()
                .find(|info| info.name == stem || info.uuid.to_string() == stem)
                .cloned()
            {
                Some(info) => Some(info),
                None => Self::read_repo_info_from_path(&path)?,
            };
            repos.push(RemoteRepoEntry { path, stem, info });
        }
        Ok(repos)
    }

    pub(crate) fn resolve_remote_repo_entry(
        &self,
        peer_id: &PeerId,
        selector: &str,
    ) -> Result<Option<RemoteRepoEntry>> {
        let selector = selector.trim_end_matches(".redb");
        let target_id = uuid::Uuid::parse_str(selector).ok();
        let mut by_id = None;
        let mut by_name = None;
        let mut by_stem = None;
        for entry in self.scan_remote_repo_entries(peer_id)? {
            if entry.stem == selector {
                by_stem = Some(entry.clone());
            }
            if let Some(info) = &entry.info {
                if info.name == selector {
                    by_name = Some(entry.clone());
                }
                if Some(info.uuid) == target_id {
                    by_id = Some(entry);
                }
            }
        }
        Ok(by_id.or(by_name).or(by_stem))
    }

    pub(crate) fn list_remote_repo_names(&self, peer_id: &PeerId) -> Result<Vec<String>> {
        let mut repos = Vec::new();
        for entry in self.scan_remote_repo_entries(peer_id)? {
            repos.push(entry.info.map(|info| info.name).unwrap_or(entry.stem));
        }
        repos.sort();
        repos.dedup();
        Ok(repos)
    }

    fn loaded_remote_repo_info(&self, peer_id: &PeerId) -> Vec<RepoInfo> {
        self.shadow_dbs
            .read()
            .unwrap()
            .get(peer_id)
            .into_iter()
            .flat_map(|repos| repos.iter())
            .map(|(repo_id, db)| {
                Self::read_repo_info_from_db(db)
                    .ok()
                    .flatten()
                    .unwrap_or(RepoInfo {
                        uuid: *repo_id,
                        name: repo_id.to_string(),
                        url: None,
                    })
            })
            .collect()
    }
}
