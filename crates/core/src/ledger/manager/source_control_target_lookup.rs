//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-selector-resolution-contract
//!   - 04_storage#internal-path-normalization
//!
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, pending_fs, staging};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
use redb::Database;
use std::collections::HashSet;

use super::source_control_target_resolution::{
    change_identity_key, has_tracked_path_only_candidates, resolve_from_entries,
};

pub(super) fn resolve_change_path(
    repo: &RepoManager,
    repo_name: &str,
    target: &ScPathTarget,
) -> Result<String> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id {
        let entries = repo.run_on_local_repo(repo_name, |db| change_entries(db, doc_id))?;
        if let Some(resolved) = resolve_from_entries(&entries, &path, Some(doc_id)) {
            return Ok(resolved);
        }
        if repo
            .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
            .is_some_and(|meta| to_forward_slash(&meta.path) == path)
        {
            return Ok(path);
        }
        return Err(anyhow!(
            "Source control target not resolved for doc {} at {}",
            doc_id,
            path
        ));
    }
    let entries = repo.list_changes_in_local_repo(repo_name)?;
    if has_tracked_path_only_candidates(&entries, &path) {
        return Err(anyhow!(
            "Tracked source control target requires document identity: {}",
            path
        ));
    }
    if let Some(resolved) = resolve_from_entries(&entries, &path, None) {
        return Ok(resolved);
    }
    Err(anyhow!(
        "Source control target not resolved for path {}",
        path
    ))
}

fn pending_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    Ok(pending_fs::list_for_doc(db, doc_id)?
        .into_iter()
        .map(|entry| ChangeEntry {
            path: entry.path,
            renamed_from: entry.renamed_from,
            doc_id: entry.doc_id,
            status: entry.change_type,
            has_conflict: entry.has_conflict,
        })
        .collect())
}

fn staged_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    Ok(staging::list_staged_entries_for_doc(db, doc_id)?
        .into_iter()
        .map(|(path, entry)| ChangeEntry {
            path,
            renamed_from: entry.renamed_from,
            doc_id: entry.doc_id,
            status: entry.status,
            has_conflict: entry.has_conflict,
        })
        .collect())
}

fn change_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    let staged = staged_entries(db, doc_id)?;
    let staged_keys: HashSet<_> = staged.iter().map(change_identity_key).collect();
    let mut changes = staged;
    changes.extend(
        pending_entries(db, doc_id)?
            .into_iter()
            .filter(|entry| !staged_keys.contains(&change_identity_key(entry))),
    );
    Ok(changes)
}
