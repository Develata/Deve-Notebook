//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 05_diff_logic#remote-import-diff-contract
//!
//! Candidate verification, dynamic blockers and current-authority review inputs.

use super::super::artifact::verify_review_artifacts;
use super::super::error::{RemoteImportError, RemoteImportResult};
use super::super::runtime::RemoteImportRuntime;
use super::super::types::{RemoteImportCandidateEntry, RemoteImportSessionRecord};
use super::super::{RemoteImportBlocker, RemoteImportCandidateRevision};
use super::baseline::{ignore_snapshot_digest, projection_locator_digest};
use super::types::RemoteImportBinding;
use crate::ledger::RepoManager;
use crate::ledger::range;
use crate::models::{GlobalSeq, RepoId};
use std::collections::BTreeSet;

pub(super) fn ensure_optional_record_revision(
    record: &RemoteImportSessionRecord,
    expected_revision: Option<RemoteImportCandidateRevision>,
) -> RemoteImportResult<()> {
    let observed = record
        .candidate
        .as_ref()
        .map(|candidate| candidate.revision);
    if observed == expected_revision {
        Ok(())
    } else {
        Err(RemoteImportError::Stale {
            session_id: record.session_id,
            blockers: Vec::new(),
        })
    }
}

pub(super) fn review_entries(
    runtime: &RemoteImportRuntime,
    record: &RemoteImportSessionRecord,
) -> RemoteImportResult<Vec<RemoteImportCandidateEntry>> {
    if record.source_snapshot.is_none() {
        return Ok(Vec::new());
    }
    verify_review_artifacts(&runtime.artifacts, record)
}

pub(super) fn dynamic_blockers(
    repo: &RepoManager,
    repo_name: &str,
    record: &RemoteImportSessionRecord,
    locator_binding: &RemoteImportBinding,
    entries: &[RemoteImportCandidateEntry],
) -> RemoteImportResult<Vec<RemoteImportBlocker>> {
    let mut blockers = entries
        .iter()
        .flat_map(|entry| entry.blockers.iter().copied())
        .collect::<BTreeSet<_>>();
    let info = repo
        .get_repo_info_for(None, Some(repo_name))
        .map_err(RemoteImportError::storage)?;
    if info.as_ref().map(|info| info.uuid) != Some(record.repo_id) {
        blockers.insert(RemoteImportBlocker::RepoMembershipMismatch);
    } else {
        let head = repo
            .run_on_local_repo(repo_name, range::get_max_seq)
            .map_err(RemoteImportError::storage)?;
        if GlobalSeq::from_storage_key(head) != record.baseline_head {
            blockers.insert(RemoteImportBlocker::LedgerHeadDrift);
        }
        let workspace_root = repo
            .local_repo_workspace_root(repo_name)
            .map_err(RemoteImportError::storage)?;
        if ignore_snapshot_digest(&workspace_root)? != record.ignore_digest {
            blockers.insert(RemoteImportBlocker::IgnoreSnapshotDrift);
        }
        if let Some(candidate) = record.candidate.as_ref()
            && projection_locator_digest(repo, record.repo_id, repo_name)?
                != candidate.locator_digest
        {
            blockers.insert(RemoteImportBlocker::LocatorBindingDrift);
        }
    }
    if locator_binding.digest() != record.locator_binding_digest {
        blockers.insert(RemoteImportBlocker::LocatorBindingDrift);
    }
    add_source_control_overlap_blockers(repo, repo_name, entries, &mut blockers)?;
    Ok(blockers.into_iter().collect())
}

fn add_source_control_overlap_blockers(
    repo: &RepoManager,
    repo_name: &str,
    entries: &[RemoteImportCandidateEntry],
    blockers: &mut BTreeSet<RemoteImportBlocker>,
) -> RemoteImportResult<()> {
    let paths = entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    add_source_control_overlap_for_paths(repo, repo_name, &paths, blockers)
}

pub(super) fn add_source_control_overlap_for_paths(
    repo: &RepoManager,
    repo_name: &str,
    paths: &BTreeSet<&str>,
    blockers: &mut BTreeSet<RemoteImportBlocker>,
) -> RemoteImportResult<()> {
    let mut doc_ids = std::collections::HashSet::new();
    for path in paths {
        if let Some(doc_id) = repo
            .get_tracked_docid_in_local_repo(repo_name, path)
            .map_err(RemoteImportError::storage)?
        {
            doc_ids.insert(doc_id.as_u128());
        }
    }
    let overlaps = |change: &crate::source_control::ChangeEntry| {
        paths.contains(change.path.as_str())
            || change
                .renamed_from
                .as_deref()
                .is_some_and(|path| paths.contains(path))
            || change
                .doc_id
                .is_some_and(|doc_id| doc_ids.contains(&doc_id.as_u128()))
    };
    if repo
        .list_pending_fs_in_local_repo(repo_name)
        .map_err(RemoteImportError::storage)?
        .iter()
        .any(overlaps)
    {
        blockers.insert(RemoteImportBlocker::PendingOverlap);
    }
    if repo
        .list_staged_in_local_repo(repo_name)
        .map_err(RemoteImportError::storage)?
        .iter()
        .any(overlaps)
    {
        blockers.insert(RemoteImportBlocker::StagedOverlap);
    }
    Ok(())
}

pub(super) fn current_content(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
) -> RemoteImportResult<Option<String>> {
    let Some(doc_id) = repo
        .get_tracked_docid_in_local_repo(repo_name, path)
        .map_err(RemoteImportError::storage)?
    else {
        return Ok(None);
    };
    let entries = repo
        .get_local_ops_in_local_repo(repo_name, doc_id)
        .map_err(RemoteImportError::storage)?
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    Ok(Some(crate::state::reconstruct_content(&entries)))
}

pub(super) fn ensure_record_repo(
    record: &RemoteImportSessionRecord,
    expected_repo_id: RepoId,
) -> RemoteImportResult<()> {
    if record.repo_id != expected_repo_id {
        return Err(RemoteImportError::ApplyFailed(
            "Remote Import session belongs to another repo".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn ensure_record_revision(
    record: &RemoteImportSessionRecord,
    expected_repo_id: RepoId,
    revision: RemoteImportCandidateRevision,
) -> RemoteImportResult<()> {
    ensure_record_repo(record, expected_repo_id)?;
    if record
        .candidate
        .as_ref()
        .map(|candidate| candidate.revision)
        != Some(revision)
    {
        return Err(RemoteImportError::InvalidState {
            session_id: record.session_id,
            state: record.state,
            expected: "exact candidate revision",
        });
    }
    Ok(())
}
