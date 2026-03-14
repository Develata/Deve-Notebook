use crate::ledger::database::{cached_database, relocate_database_path};
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct RemoteRepoEntry {
    pub path: PathBuf,
    pub stem: String,
    pub info: Option<RepoInfo>,
}

struct RemoteRepoCatalogInfo {
    info: RepoInfo,
    write_back: bool,
}

impl RepoManager {
    fn repair_remote_repo_catalog(&self, peer_id: &PeerId) -> Result<()> {
        let peer_dir = self.remotes_dir().join(peer_id.to_filename());
        if !peer_dir.exists() {
            return Ok(());
        }
        let mut paths = std::fs::read_dir(&peer_dir)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("redb"))
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let Some(repair) = self.repaired_remote_repo_info(&path, &stem)? else {
                continue;
            };
            let desired = self.allocate_remote_repo_path(peer_id, &repair.info)?;
            let target = if desired != path {
                relocate_database_path(&path, &desired);
                std::fs::rename(&path, &desired)?;
                desired
            } else {
                path
            };
            if repair.write_back {
                let db = cached_database(&target)?;
                Self::write_repo_info_to_db(db.as_ref(), &repair.info)?;
            }
        }
        Ok(())
    }

    pub(crate) fn scan_remote_repo_entries(
        &self,
        peer_id: &PeerId,
    ) -> Result<Vec<RemoteRepoEntry>> {
        self.repair_remote_repo_catalog(peer_id)?;
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
                None => self
                    .repaired_remote_repo_info(&path, &stem)?
                    .map(|repair| repair.info),
            };
            repos.push(RemoteRepoEntry { path, stem, info });
        }
        Ok(repos)
    }

    fn repaired_remote_repo_info(
        &self,
        path: &Path,
        stem: &str,
    ) -> Result<Option<RemoteRepoCatalogInfo>> {
        let original = Self::read_repo_info_from_path(path)?;
        let Some(mut info) = original.clone().or_else(|| {
            uuid::Uuid::parse_str(stem).ok().map(|repo_id| {
                self.get_local_repo_info_by_id_without_repair(repo_id)
                    .ok()
                    .flatten()
                    .unwrap_or(RepoInfo {
                        uuid: repo_id,
                        name: stem.to_string(),
                        url: Some(format!("urn:uuid:{}", repo_id)),
                    })
            })
        }) else {
            return Ok(None);
        };
        let mut write_back = original.is_none();
        if info.name.trim().is_empty() {
            info.name = stem.to_string();
            write_back = true;
        }
        if info.url.is_none() {
            info.url = self
                .get_local_repo_info_by_id_without_repair(info.uuid)?
                .and_then(|local| local.url)
                .or_else(|| Some(format!("urn:uuid:{}", info.uuid)));
            write_back = true;
        }
        if original.as_ref() != Some(&info) {
            write_back = true;
        }
        Ok(Some(RemoteRepoCatalogInfo { info, write_back }))
    }

    pub(crate) fn resolve_remote_repo_entry(
        &self,
        peer_id: &PeerId,
        selector: &str,
    ) -> Result<Option<RemoteRepoEntry>> {
        let selector = selector.trim_end_matches(".redb");
        let target_id = uuid::Uuid::parse_str(selector).ok();
        let mut by_id = None;
        let mut by_stem = None;
        let mut by_name = Vec::new();
        for entry in self.scan_remote_repo_entries(peer_id)? {
            if entry.stem == selector {
                by_stem = Some(entry.clone());
            }
            if let Some(info) = &entry.info {
                if info.name == selector {
                    by_name.push(entry.clone());
                }
                if Some(info.uuid) == target_id {
                    by_id = Some(entry);
                }
            }
        }
        Ok(by_id.or(by_stem).or_else(|| {
            if by_name.len() == 1 {
                by_name.into_iter().next()
            } else {
                None
            }
        }))
    }

    pub fn find_remote_repo_selector_by_id(
        &self,
        peer_id: &PeerId,
        repo_id: uuid::Uuid,
    ) -> Result<Option<String>> {
        Ok(self
            .resolve_remote_repo_entry(peer_id, &repo_id.to_string())?
            .map(|entry| entry.stem))
    }

    pub(crate) fn list_remote_repo_names(&self, peer_id: &PeerId) -> Result<Vec<String>> {
        let entries = self.scan_remote_repo_entries(peer_id)?;
        let mut counts = HashMap::<String, usize>::new();
        let mut named = Vec::new();
        for entry in entries {
            let display = self.remote_repo_display_name(&entry);
            *counts.entry(display.clone()).or_default() += 1;
            named.push((entry, display));
        }
        let mut repos: Vec<_> = named
            .into_iter()
            .map(|(entry, display)| {
                if counts.get(&display).copied().unwrap_or(0) > 1 {
                    entry.stem
                } else {
                    display
                }
            })
            .collect();
        repos.sort();
        Ok(repos)
    }

    fn remote_repo_display_name(&self, entry: &RemoteRepoEntry) -> String {
        entry
            .info
            .as_ref()
            .map(|info| info.name.clone())
            .unwrap_or_else(|| entry.stem.clone())
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
