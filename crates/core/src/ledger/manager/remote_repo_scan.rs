use crate::ledger::database::{cached_database, relocate_database_path};
use crate::ledger::manager::remote_repo_scan_entry::{RemoteRepoCatalogInfo, RemoteRepoEntry};
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};
use std::path::Path;

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
        let mut repairs = Vec::new();
        for path in paths {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let repair = match self.repaired_remote_repo_info(&path, &stem) {
                Ok(Some(repair)) => repair,
                Ok(None) => continue,
                Err(err) => {
                    tracing::warn!(
                        "Skipping unreadable shadow repo {} during repair: {:?}",
                        stem,
                        err
                    );
                    continue;
                }
            };
            repairs.push((path, repair));
        }
        let duplicate_ids = duplicate_catalog_ids(
            repairs
                .iter()
                .map(|(_, repair)| repair.info.uuid)
                .collect::<Vec<_>>(),
        );
        for (path, repair) in repairs {
            if duplicate_ids.contains(&repair.info.uuid) {
                tracing::warn!(
                    "Skipping duplicate shadow UUID during repair: peer={}, uuid={}, path={:?}",
                    peer_id,
                    repair.info.uuid,
                    path
                );
                continue;
            }
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
        self.scan_remote_repo_entries_without_repair(peer_id)
    }

    pub(crate) fn scan_remote_repo_entries_without_repair(
        &self,
        peer_id: &PeerId,
    ) -> Result<Vec<RemoteRepoEntry>> {
        let loaded = self.loaded_remote_repo_info(peer_id);
        let mut loaded_by_id = HashMap::new();
        let mut loaded_name_counts = HashMap::<String, usize>::new();
        for info in &loaded {
            loaded_by_id.insert(info.uuid, info.clone());
            *loaded_name_counts.entry(info.name.clone()).or_default() += 1;
        }
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
            let info = match uuid::Uuid::parse_str(&stem)
                .ok()
                .and_then(|repo_id| loaded_by_id.get(&repo_id).cloned())
                .or_else(|| {
                    (loaded_name_counts.get(&stem).copied() == Some(1))
                        .then(|| loaded.iter().find(|info| info.name == stem).cloned())
                        .flatten()
                }) {
                Some(info) => Some(info),
                None => match Self::read_remote_repo_info_without_repair(&path, &stem) {
                    Ok(info) => info,
                    Err(err) => {
                        tracing::warn!(
                            "Keeping shadow repo entry {} without metadata during pure scan: {:?}",
                            stem,
                            err
                        );
                        None
                    }
                },
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
            uuid::Uuid::parse_str(stem).ok().map(|repo_id| RepoInfo {
                uuid: repo_id,
                name: stem.to_string(),
                url: None,
            })
        }) else {
            return Ok(None);
        };
        let mut write_back = original.is_none();
        if info.name.trim().is_empty() {
            info.name = stem.to_string();
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
        let entries = self.scan_remote_repo_entries(peer_id)?;
        let duplicate_ids = duplicate_entry_ids(&entries);
        let mut by_id = Vec::new();
        let mut by_stem = Vec::new();
        let mut by_name = Vec::new();
        for entry in entries {
            let Some(info) = &entry.info else {
                continue;
            };
            if entry.stem == selector {
                by_stem.push(entry.clone());
            }
            if info.name == selector {
                by_name.push(entry.clone());
            }
            if Some(info.uuid) == target_id {
                by_id.push(entry);
            }
        }
        reject_duplicate_remote_matches(selector, &by_id, &duplicate_ids)?;
        if let Some(entry) = single_remote_entry(by_id) {
            return Ok(Some(entry));
        }
        reject_duplicate_remote_matches(selector, &by_stem, &duplicate_ids)?;
        if let Some(entry) = single_remote_entry(by_stem) {
            return Ok(Some(entry));
        }
        reject_duplicate_remote_matches(selector, &by_name, &duplicate_ids)?;
        Ok(single_remote_entry(by_name))
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

    pub fn find_remote_repo_selector(
        &self,
        peer_id: &PeerId,
        selector: &str,
    ) -> Result<Option<String>> {
        Ok(self
            .resolve_remote_repo_entry(peer_id, selector)?
            .map(|entry| entry.stem))
    }

    pub(crate) fn list_remote_repo_names(&self, peer_id: &PeerId) -> Result<Vec<String>> {
        let entries = self
            .scan_remote_repo_entries(peer_id)?
            .into_iter()
            .collect::<Vec<_>>();
        let duplicate_ids = duplicate_entry_ids(&entries);
        let entries = entries
            .into_iter()
            .filter(|entry| {
                entry.is_readable()
                    && entry
                        .info
                        .as_ref()
                        .is_some_and(|info| !duplicate_ids.contains(&info.uuid))
            })
            .collect::<Vec<_>>();
        let mut counts = HashMap::<String, usize>::new();
        let mut named = Vec::new();
        for entry in entries {
            let display = entry.display_name();
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

    fn read_remote_repo_info_without_repair(path: &Path, stem: &str) -> Result<Option<RepoInfo>> {
        if let Some(info) = Self::read_repo_info_from_path(path)? {
            return Ok(Some(info));
        }
        Ok(uuid::Uuid::parse_str(stem).ok().map(|repo_id| RepoInfo {
            uuid: repo_id,
            name: stem.to_string(),
            url: None,
        }))
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

fn duplicate_catalog_ids(ids: Vec<uuid::Uuid>) -> HashSet<uuid::Uuid> {
    let mut counts = HashMap::<uuid::Uuid, usize>::new();
    for id in ids {
        *counts.entry(id).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect()
}

fn duplicate_entry_ids(entries: &[RemoteRepoEntry]) -> HashSet<uuid::Uuid> {
    duplicate_catalog_ids(
        entries
            .iter()
            .filter_map(|entry| entry.info.as_ref().map(|info| info.uuid))
            .collect(),
    )
}

fn reject_duplicate_remote_matches(
    selector: &str,
    matches: &[RemoteRepoEntry],
    duplicate_ids: &HashSet<uuid::Uuid>,
) -> Result<()> {
    if matches.iter().any(|entry| {
        entry
            .info
            .as_ref()
            .is_some_and(|info| duplicate_ids.contains(&info.uuid))
    }) {
        return Err(anyhow!("ambiguous remote repository selector: {}", selector));
    }
    Ok(())
}

fn single_remote_entry(entries: Vec<RemoteRepoEntry>) -> Option<RemoteRepoEntry> {
    (entries.len() == 1).then(|| entries.into_iter().next()).flatten()
}
