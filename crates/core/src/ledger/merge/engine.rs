// crates\core\src\ledger\merge\engine.rs
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!
// ---------------------------------------------------------------
// 模块：三路合并引擎
// 作用：对已经由 MergeBaseCheckpoint 证明的三方内容执行 3-Way Merge
// 功能：冲突检测、冲突片段构建、合并结果输出
// ---------------------------------------------------------------

use super::diff::{apply_edits, diff_to_edits};
use super::region::merge_regions;
use super::types::MergeResult;

pub struct MergeEngine;

impl MergeEngine {
    /// 执行 3-Way Merge
    pub fn merge_commits(base: &str, local: &str, remote: &str) -> MergeResult {
        if local == remote {
            return MergeResult::Success(local.to_string());
        }
        if base == local {
            return MergeResult::Success(remote.to_string());
        }
        if base == remote {
            return MergeResult::Success(local.to_string());
        }

        let mut local_edits = diff_to_edits(base, local);
        let mut remote_edits = diff_to_edits(base, remote);
        local_edits.sort_by_key(|e| e.start);
        remote_edits.sort_by_key(|e| e.start);

        let outcome = merge_regions(base, &local_edits, &remote_edits);

        if !outcome.conflicts.is_empty() {
            return MergeResult::Conflict {
                base: base.to_string(),
                local: local.to_string(),
                remote: remote.to_string(),
                conflicts: outcome.conflicts,
            };
        }

        let merged = apply_edits(base, &outcome.merged_edits);
        MergeResult::Success(merged)
    }
}
