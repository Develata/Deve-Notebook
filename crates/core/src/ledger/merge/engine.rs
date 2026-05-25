// crates\core\src\ledger\merge\engine.rs
//! plan_ref:
//!   - 03_storage#facts-partition
//!   - 10_rendering#document-authority-bridge
//!
// ---------------------------------------------------------------
// 模块：三路合并引擎
// 作用：执行 LCA 计算、状态重建与 3-Way Merge
// 功能：冲突检测、冲突片段构建、合并结果输出
// ---------------------------------------------------------------

use crate::models::{DocId, LedgerEntry, VersionVector};

use super::diff::{apply_edits, diff_to_edits};
use super::region::merge_regions;
use super::types::MergeResult;

pub struct MergeEngine;

impl MergeEngine {
    /// 计算本地与远端的 LCA 版本向量
    pub fn find_lca(local_vv: &VersionVector, remote_vv: &VersionVector) -> VersionVector {
        // LCA = 两个向量的逐分量最小值
        local_vv.intersection(remote_vv)
    }

    /// 在指定 VersionVector 上重建文档内容
    ///
    /// 复杂点：需要过滤出对 LCA 可见的操作
    pub fn reconstruct_state_at(
        _doc_id: DocId,
        all_ops: &[LedgerEntry],
        at_vv: &VersionVector,
    ) -> String {
        let visible_ops: Vec<LedgerEntry> = all_ops
            .iter()
            .filter(|entry| {
                let max_seq = at_vv.get(&entry.peer_id);
                entry.seq <= max_seq
            })
            .cloned()
            .collect();

        crate::state::reconstruct_content(&visible_ops)
    }

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
