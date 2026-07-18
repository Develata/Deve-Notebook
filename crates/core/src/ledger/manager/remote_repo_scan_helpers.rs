//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-catalog-repair-contract
//!   - 04_repository#repo-selector-resolution-contract

use crate::ledger::manager::remote_repo_scan_entry::{RemoteRepoCatalogInfo, RemoteRepoEntry};
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn checked_remote_peer_dir(
    manager: &RepoManager,
    peer_id: &PeerId,
    action: &str,
) -> Result<PathBuf> {
    let peer_dir = manager.checked_remotes_dir()?.join(peer_id.to_filename());
    match peer_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(anyhow!(
                "Broken shadow peer {} while {}: directory missing",
                peer_id,
                action
            ));
        }
        Err(err) => {
            return Err(anyhow!(
                "Failed to stat remote peer directory {:?} while {}: {}",
                peer_dir,
                action,
                err
            ));
        }
    }
    if !std::fs::metadata(&peer_dir)
        .map_err(|err| {
            anyhow!(
                "Failed to read remote peer directory metadata {:?} while {}: {}",
                peer_dir,
                action,
                err
            )
        })?
        .is_dir()
    {
        return Err(anyhow!(
            "Broken shadow peer {} while {}: expected directory",
            peer_id,
            action
        ));
    }
    Ok(peer_dir)
}

pub(super) fn read_remote_repo_info_without_repair(
    _repo: &RepoManager,
    path: &Path,
    _stem: &str,
) -> Result<Option<RepoInfo>> {
    RepoManager::read_repo_info_from_path(path)
}

pub(super) fn scanned_remote_repo_info(
    repo: &RepoManager,
    path: &Path,
    stem: &str,
) -> Result<Option<RepoInfo>> {
    read_remote_repo_info_without_repair(repo, path, stem)
}

pub(super) fn repaired_remote_repo_info(path: &Path, stem: &str) -> Result<RemoteRepoCatalogInfo> {
    let original = RepoManager::read_repo_info_from_path(path)?;
    let Some(mut info) = original.clone().or_else(|| {
        uuid::Uuid::parse_str(stem).ok().map(|repo_id| RepoInfo {
            uuid: repo_id,
            name: stem.to_string(),
            url: None,
        })
    }) else {
        return Err(anyhow!("repo metadata missing and file stem is not a UUID"));
    };
    let mut write_back = original.is_none();
    if info.name.trim().is_empty() {
        info.name = if uuid::Uuid::parse_str(stem).ok() == Some(info.uuid) {
            stem.to_string()
        } else {
            info.uuid.to_string()
        };
        write_back = true;
    }
    if original.as_ref() != Some(&info) {
        write_back = true;
    }
    Ok(RemoteRepoCatalogInfo { info, write_back })
}

pub(super) fn duplicate_catalog_ids(ids: Vec<uuid::Uuid>) -> HashSet<uuid::Uuid> {
    let mut counts = HashMap::<uuid::Uuid, usize>::new();
    for id in ids {
        *counts.entry(id).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect()
}

pub(super) fn duplicate_entry_ids(entries: &[RemoteRepoEntry]) -> HashSet<uuid::Uuid> {
    duplicate_catalog_ids(
        entries
            .iter()
            .filter_map(|entry| entry.info.as_ref().map(|info| info.uuid))
            .collect(),
    )
}

pub(super) fn validate_scanned_remote_entries(
    peer_id: &PeerId,
    entries: &[RemoteRepoEntry],
    action: &str,
) -> Result<()> {
    if let Some(entry) = entries.iter().find(|entry| !entry.is_readable()) {
        return Err(anyhow!(
            "Broken shadow repo {} for peer {} while {}",
            entry.stem,
            peer_id,
            action
        ));
    }
    let duplicate_ids = duplicate_entry_ids(entries);
    if !duplicate_ids.is_empty() {
        let mut duplicate_ids = duplicate_ids
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>();
        duplicate_ids.sort();
        return Err(anyhow!(
            "Broken shadow peer {} while {}: duplicate remote repository UUIDs: {}",
            peer_id,
            action,
            duplicate_ids.join(", ")
        ));
    }
    if let Some((entry, info)) = entries.iter().find_map(|entry| {
        entry
            .info
            .as_ref()
            .filter(|info| entry.stem != info.uuid.to_string())
            .map(|info| (entry, info))
    }) {
        return Err(anyhow!(
            "Broken shadow repo {} for peer {} while {}: physical stem must equal RepoId {}",
            entry.stem,
            peer_id,
            action,
            info.uuid
        ));
    }
    Ok(())
}

pub(super) fn reject_duplicate_remote_matches(
    selector: &str,
    matches: &[RemoteRepoEntry],
    duplicate_ids: &HashSet<uuid::Uuid>,
) -> Result<()> {
    if matches.len() > 1 {
        return Err(anyhow!(
            "ambiguous remote repository selector: {}",
            selector
        ));
    }
    if matches.iter().any(|entry| {
        entry
            .info
            .as_ref()
            .is_some_and(|info| duplicate_ids.contains(&info.uuid))
    }) {
        return Err(anyhow!(
            "ambiguous remote repository selector: {}",
            selector
        ));
    }
    Ok(())
}

pub(super) fn single_remote_entry(entries: Vec<RemoteRepoEntry>) -> Option<RemoteRepoEntry> {
    (entries.len() == 1)
        .then(|| entries.into_iter().next())
        .flatten()
}
