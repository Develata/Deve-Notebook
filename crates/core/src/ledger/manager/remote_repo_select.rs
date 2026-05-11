//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract

use crate::ledger::manager::remote_repo_scan_entry::RemoteRepoEntry;
use crate::ledger::manager::remote_repo_scan_helpers::{
    duplicate_entry_ids, reject_duplicate_remote_matches, single_remote_entry,
};
use crate::ledger::manager::repo_catalog_runtime::RepoCatalogRuntime;
use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use anyhow::{Result, anyhow};

impl<'a> RepoCatalogRuntime<'a> {
    pub(crate) fn resolve_remote_repo_entry(
        &self,
        peer_id: &PeerId,
        selector: &str,
    ) -> Result<Option<RemoteRepoEntry>> {
        let selector = selector.trim_end_matches(".redb");
        let target_id = uuid::Uuid::parse_str(selector).ok();
        let entries = self.scan_remote_repo_entries(peer_id)?;
        if let Some(entry) = entries.iter().find(|entry| !entry.is_readable()) {
            return Err(anyhow!(
                "Broken shadow repo {} for peer {} while resolving selector",
                entry.stem,
                peer_id
            ));
        }
        let duplicate_ids = duplicate_entry_ids(&entries);
        let mut by_id = Vec::new();
        let mut by_stem = Vec::new();
        let mut by_name = Vec::new();
        for entry in entries {
            let info = entry.info.as_ref().expect("validated readable");
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
        reject_duplicate_remote_matches(selector, &by_name, &duplicate_ids)?;
        Ok(single_remote_entry(by_id))
    }

    pub(crate) fn find_remote_repo_selector_by_id(
        &self,
        peer_id: &PeerId,
        repo_id: uuid::Uuid,
    ) -> Result<Option<String>> {
        Ok(self
            .manager
            .resolve_remote_repo_entry_by_id(peer_id, repo_id)?
            .filter(|entry| entry.is_switchable())
            .map(|entry| entry.stem))
    }

    pub(crate) fn find_remote_repo_selector(
        &self,
        peer_id: &PeerId,
        selector: &str,
    ) -> Result<Option<String>> {
        Ok(self
            .resolve_remote_repo_entry(peer_id, selector)?
            .filter(|entry| entry.is_switchable())
            .map(|entry| entry.stem))
    }

    pub(crate) fn has_remote_display_name(&self, peer_id: &PeerId, raw_name: &str) -> Result<bool> {
        let entries = self.scan_remote_repo_entries(peer_id)?;
        if let Some(entry) = entries.iter().find(|entry| !entry.is_readable()) {
            return Err(anyhow!(
                "Broken shadow repo {} for peer {} while checking display name",
                entry.stem,
                peer_id
            ));
        }
        Ok(entries.into_iter().any(|entry| {
            entry
                .info
                .as_ref()
                .is_some_and(|info| entry.is_switchable() && info.name == raw_name)
        }))
    }

    pub(crate) fn list_remote_repo_names(&self, peer_id: &PeerId) -> Result<Vec<String>> {
        let entries = self.scan_remote_repo_entries(peer_id)?;
        if let Some(entry) = entries.iter().find(|entry| !entry.is_readable()) {
            return Err(anyhow!(
                "Broken shadow repo {} for peer {} while listing remote repos",
                entry.stem,
                peer_id
            ));
        }
        let duplicate_ids = duplicate_entry_ids(&entries);
        let mut repos: Vec<_> = entries
            .into_iter()
            .filter(|entry| {
                entry.info.as_ref().is_some_and(|info| {
                    entry.is_switchable() && !duplicate_ids.contains(&info.uuid)
                })
            })
            .map(|entry| entry.stem)
            .collect();
        repos.sort();
        Ok(repos)
    }
}

impl RepoManager {
    pub(crate) fn resolve_remote_repo_entry(
        &self,
        peer_id: &PeerId,
        selector: &str,
    ) -> Result<Option<RemoteRepoEntry>> {
        self.repo_catalog_runtime()
            .resolve_remote_repo_entry(peer_id, selector)
    }

    pub fn find_remote_repo_selector_by_id(
        &self,
        peer_id: &PeerId,
        repo_id: uuid::Uuid,
    ) -> Result<Option<String>> {
        self.repo_catalog_runtime()
            .find_remote_repo_selector_by_id(peer_id, repo_id)
    }

    pub fn find_remote_repo_selector(
        &self,
        peer_id: &PeerId,
        selector: &str,
    ) -> Result<Option<String>> {
        self.repo_catalog_runtime()
            .find_remote_repo_selector(peer_id, selector)
    }

    pub fn has_remote_display_name(&self, peer_id: &PeerId, raw_name: &str) -> Result<bool> {
        self.repo_catalog_runtime()
            .has_remote_display_name(peer_id, raw_name)
    }
}
