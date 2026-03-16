#[path = "remote_repo_scan_helpers.rs"]
mod helpers;

use self::helpers::{
    duplicate_catalog_ids, duplicate_entry_ids, reject_duplicate_remote_matches,
    repaired_remote_repo_info, resolve_remote_repo_entry_by_id, scanned_remote_repo_info,
    single_remote_entry,
};
use crate::ledger::database::{cached_database, relocate_database_path};
use crate::ledger::manager::remote_repo_scan_entry::RemoteRepoEntry;
use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use anyhow::Result;
use std::collections::HashMap;

impl RepoManager {
    fn repair_remote_repo_catalog(&self, peer_id: &PeerId) -> Result<()> {
        let peer_dir = self.remotes_dir().join(peer_id.to_filename());
        if !peer_dir.exists() {
            return Ok(());
        }
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&peer_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("redb") {
                paths.push(path);
            }
        }
        paths.sort();
        let mut repairs = Vec::new();
        for path in paths {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let repair = match repaired_remote_repo_info(&path, &stem) {
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
            let info = scanned_remote_repo_info(self, &path, &stem);
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
        reject_duplicate_remote_matches(selector, &by_stem, &duplicate_ids)?;
        if let Some(entry) = single_remote_entry(by_stem) {
            return Ok(Some(entry));
        }
        reject_duplicate_remote_matches(selector, &by_id, &duplicate_ids)?;
        if let Some(entry) = single_remote_entry(by_id) {
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
            .resolve_remote_repo_entry_by_id(peer_id, repo_id)?
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

    pub fn has_remote_display_name(&self, peer_id: &PeerId, raw_name: &str) -> Result<bool> {
        Ok(self
            .scan_remote_repo_entries(peer_id)?
            .into_iter()
            .any(|entry| {
                entry
                    .info
                    .as_ref()
                    .is_some_and(|info| info.name == raw_name)
            }))
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
}

impl RepoManager {
    pub(crate) fn resolve_remote_repo_entry_by_id(
        &self,
        peer_id: &PeerId,
        repo_id: uuid::Uuid,
    ) -> Result<Option<RemoteRepoEntry>> {
        resolve_remote_repo_entry_by_id(self, peer_id, repo_id)
    }
}
