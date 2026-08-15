//! plan_ref:
//!   - 03_storage/projection#projection-contract

use super::DriftKind;
use crate::source_control::{ChangeStatus, pending_fs, staging};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
pub struct DiffEvidence<'a> {
    pub path: &'a str,
    pub kind: DriftKind,
    pub workspace_hash: Option<&'a str>,
}

#[derive(Default)]
pub struct ExplanationIndex {
    missing_paths: HashSet<String>,
    inserted_hashes: HashMap<String, HashSet<String>>,
    modified_hashes: HashMap<String, HashSet<String>>,
}

impl ExplanationIndex {
    pub fn new(
        pending: &[pending_fs::PendingFsEntry],
        staged: &[(String, staging::StagedEntry)],
    ) -> Self {
        let mut index = Self::default();
        for entry in pending {
            index.observe(
                entry.change_type,
                &entry.path,
                entry.renamed_from.as_deref(),
                &entry.content_hash,
            );
        }
        for (path, entry) in staged {
            index.observe(
                entry.status,
                path,
                entry.renamed_from.as_deref(),
                &entry.content_hash,
            );
        }
        index
    }

    pub fn is_explained(&self, diff: DiffEvidence<'_>) -> bool {
        match diff.kind {
            DriftKind::MissingOnDisk => self.missing_paths.contains(diff.path),
            DriftKind::UnexpectedOnDisk => diff.workspace_hash.is_some_and(|hash| {
                self.inserted_hashes
                    .get(diff.path)
                    .is_some_and(|hashes| hashes.contains(hash))
            }),
            DriftKind::ContentMismatch => diff.workspace_hash.is_some_and(|hash| {
                self.modified_hashes
                    .get(diff.path)
                    .is_some_and(|hashes| hashes.contains(hash))
            }),
        }
    }

    fn observe(
        &mut self,
        status: ChangeStatus,
        path: &str,
        renamed_from: Option<&str>,
        hash: &str,
    ) {
        match status {
            ChangeStatus::Deleted => {
                self.missing_paths.insert(path.to_string());
            }
            ChangeStatus::Renamed => {
                if let Some(previous) = renamed_from {
                    self.missing_paths.insert(previous.to_string());
                }
                insert_hash(&mut self.inserted_hashes, path, hash);
                insert_hash(&mut self.modified_hashes, path, hash);
            }
            ChangeStatus::Added => insert_hash(&mut self.inserted_hashes, path, hash),
            ChangeStatus::Modified => insert_hash(&mut self.modified_hashes, path, hash),
        }
    }
}

fn insert_hash(index: &mut HashMap<String, HashSet<String>>, path: &str, hash: &str) {
    index
        .entry(path.to_string())
        .or_default()
        .insert(hash.to_string());
}
