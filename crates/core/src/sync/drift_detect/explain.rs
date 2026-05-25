//! plan_ref:
//!   - 03_storage/projection#projection-contract

use super::DriftKind;
use crate::source_control::{ChangeStatus, pending_fs, staging};

#[derive(Clone, Copy)]
pub struct DiffEvidence<'a> {
    pub path: &'a str,
    pub kind: DriftKind,
    pub workspace_hash: Option<&'a str>,
}

pub fn is_explained(
    diff: DiffEvidence,
    pending: &[pending_fs::PendingFsEntry],
    staged: &[(String, staging::StagedEntry)],
) -> bool {
    pending.iter().any(|entry| matches_pending(diff, entry))
        || staged
            .iter()
            .any(|(path, entry)| matches_staged(diff, path, entry))
}

fn matches_pending(diff: DiffEvidence, entry: &pending_fs::PendingFsEntry) -> bool {
    match diff.kind {
        DriftKind::MissingOnDisk => match entry.change_type {
            ChangeStatus::Deleted => entry.path == diff.path,
            ChangeStatus::Renamed => entry.renamed_from.as_deref() == Some(diff.path),
            _ => false,
        },
        DriftKind::UnexpectedOnDisk => {
            matches_insert_like(diff, entry.change_type, &entry.path, &entry.content_hash)
        }
        DriftKind::ContentMismatch => {
            matches_modify_like(diff, entry.change_type, &entry.path, &entry.content_hash)
        }
    }
}

fn matches_staged(diff: DiffEvidence, path: &str, entry: &staging::StagedEntry) -> bool {
    match diff.kind {
        DriftKind::MissingOnDisk => match entry.status {
            ChangeStatus::Deleted => path == diff.path,
            ChangeStatus::Renamed => entry.renamed_from.as_deref() == Some(diff.path),
            _ => false,
        },
        DriftKind::UnexpectedOnDisk => {
            matches_insert_like(diff, entry.status, path, &entry.content_hash)
        }
        DriftKind::ContentMismatch => {
            matches_modify_like(diff, entry.status, path, &entry.content_hash)
        }
    }
}

fn matches_insert_like(diff: DiffEvidence, status: ChangeStatus, path: &str, hash: &str) -> bool {
    matches!(status, ChangeStatus::Added | ChangeStatus::Renamed)
        && path == diff.path
        && diff
            .workspace_hash
            .is_some_and(|candidate| candidate == hash)
}

fn matches_modify_like(diff: DiffEvidence, status: ChangeStatus, path: &str, hash: &str) -> bool {
    matches!(status, ChangeStatus::Modified | ChangeStatus::Renamed)
        && path == diff.path
        && diff
            .workspace_hash
            .is_some_and(|candidate| candidate == hash)
}
