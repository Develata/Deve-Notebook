use crate::ledger::manager::remote_repo_scan_entry::{RemoteRepoCatalogInfo, RemoteRepoEntry};
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::PeerId;
use anyhow::{Result, anyhow};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) fn read_remote_repo_info_without_repair(
    _repo: &RepoManager,
    path: &Path,
    stem: &str,
) -> Result<Option<RepoInfo>> {
    if let Some(info) = RepoManager::read_repo_info_from_path(path)? {
        return Ok(Some(info));
    }
    Ok(uuid::Uuid::parse_str(stem).ok().map(|repo_id| RepoInfo {
        uuid: repo_id,
        name: stem.to_string(),
        url: None,
    }))
}

pub(super) fn loaded_remote_repo_info(repo: &RepoManager, peer_id: &PeerId) -> Vec<RepoInfo> {
    repo.shadow_dbs
        .read()
        .unwrap()
        .get(peer_id)
        .into_iter()
        .flat_map(|repos| repos.iter())
        .map(|(repo_id, db)| {
            RepoManager::read_repo_info_from_db(db)
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

pub(super) fn repaired_remote_repo_info(
    path: &Path,
    stem: &str,
) -> Result<Option<RemoteRepoCatalogInfo>> {
    let original = RepoManager::read_repo_info_from_path(path)?;
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
    Ok(Some(RemoteRepoCatalogInfo { info, write_back }))
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

pub(super) fn reject_duplicate_remote_matches(
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

pub(super) fn resolve_remote_repo_entry_by_id(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
) -> Result<Option<RemoteRepoEntry>> {
    let entries = repo.scan_remote_repo_entries(peer_id)?;
    let duplicate_ids = duplicate_entry_ids(&entries);
    let matches = entries
        .into_iter()
        .filter(|entry| entry.info.as_ref().is_some_and(|info| info.uuid == repo_id))
        .collect::<Vec<_>>();
    reject_duplicate_remote_matches(&repo_id.to_string(), &matches, &duplicate_ids)?;
    Ok(single_remote_entry(matches))
}
