// crates\core\src\ledger
//! # 3-Way Merge Engine
//!
//! implements the "Git-style" 3-way merge logic and LCA calculation.

use crate::models::{DocId, LedgerEntry, RepoId};
use crate::sync::vector::VersionVector;
use anyhow::Result;

/// Result of a merge operation
#[derive(Debug)]
pub enum MergeResult {
    /// Auto-merged successfully
    Success(String),
    /// Conflict detected
    Conflict {
        base: String,
        local: String,
        remote: String,
        conflicts: Vec<ConflictHunk>,
    },
}

#[derive(Debug)]
pub struct ConflictHunk {
    pub start_line: usize,
    pub length: usize,
    // ...
}

pub struct MergeEngine;

impl MergeEngine {
    /// Find the Lowest Common Ancestor (LCA) Version Vector between local and remote
    pub fn find_lca(local_vv: &VersionVector, remote_vv: &VersionVector) -> VersionVector {
        // LCA is the intersection (min) of the two vectors.
        local_vv.intersection(remote_vv)
    }

    /// Reconstruct the document state at a specific Version Vector (LCA).
    /// This requires retrieving ALL ops for the document and filtering those
    /// that are "visible" to the LCA vector.
    pub fn reconstruct_state_at(
        doc_id: DocId,
        all_ops: &[LedgerEntry],
        at_vv: &VersionVector,
    ) -> String {
        // Filter ops: visible if op.seq <= at_vv.get(op.peer_id)
        let mut visible_ops: Vec<LedgerEntry> = all_ops
            .iter()
            .filter(|entry| {
                let max_seq = at_vv.get(&entry.peer_id);
                entry.seq <= max_seq
            })
            .cloned()
            .collect();

        // Sort by timestamp (or causal order if available).
        // Since we don't have Lamport Clock explicit here, we rely on timestamp + arbitration or original order?
        // If `all_ops` is already sorted by GlobalSeq, we should preserve that relative order?
        // Yes, preserving relative order of filtered ops is best if they were causal.
        // However, if we merge two branches, the GlobalSeq might be interleaved.
        // Assuming `all_ops` comes from `get_ops` which is sorted by GlobalSeq.
        crate::state::reconstruct_content(&visible_ops)
    }

    /// Perform a 3-way merge
    pub fn merge_commits(
        base: &str,
        local: &str,
        remote: &str,
    ) -> MergeResult {
        // Naive line-based 3-way merge logic or specialized algorithm.
        // For simplicity and correctness, we often rely on:
        // Diff(Base -> Local)
        // Diff(Base -> Remote)
        // Combine chunks.
        
        // Since `dissimilar` is character/word based by default depending on usage,
        // typically for code/text we want line-based.
        // However, `state::compute_diff` uses `dissimilar::diff` which handles chars.
        // For 3-Way Merge, Line-based is safer for conflicts.
        // Implementing a full diff3 is complex.
        
        // Let's implement a simplified check:
        // If Local == Remote, return Local.
        // If Base == Local, return Remote (Fast Forward).
        // If Base == Remote, return Local (Fast Forward).
        // If diff chunks don't overlap, apply both.
        // If overlap, return Conflict.
        
        if local == remote {
            return MergeResult::Success(local.to_string());
        }
        if base == local {
            return MergeResult::Success(remote.to_string());
        }
        if base == remote {
            return MergeResult::Success(local.to_string());
        }

        // Real 3-way merge is hard without a library.
        // For this prototype, we will detect conflict if both changed.
        
        // TODO: Integrate a proper diff3 library (e.g. `merge` crate or port logic).
        // Given constraints and `dissimilar` usage:
        // We will mark it as "Manual Merge Required" if simple heuristics fail.
        
        // Heuristic:
        let diff_local = crate::state::compute_diff(base, local);
        let diff_remote = crate::state::compute_diff(base, remote);
        
        // If we can't auto-merge comfortably:
        MergeResult::Conflict {
            base: base.to_string(),
            local: local.to_string(),
            remote: remote.to_string(),
            conflicts: vec![], // Details omitted for now
        }
}
    }
