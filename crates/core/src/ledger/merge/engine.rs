// crates\core\src\ledger\merge\engine.rs
// ---------------------------------------------------------------
// 模块：三路合并引擎
// 作用：执行 LCA 计算、状态重建与 3-Way Merge
// 功能：冲突检测、冲突片段构建、合并结果输出
// ---------------------------------------------------------------

use crate::models::{DocId, LedgerEntry};
use crate::sync::vector::VersionVector;

use super::diff::{apply_edits, diff_to_edits, edits_equivalent, edits_overlap, Edit};
use super::types::{ConflictHunk, MergeResult};

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

        let mut merged_edits: Vec<Edit> = Vec::new();
        let mut conflicts: Vec<ConflictHunk> = Vec::new();
        let mut i = 0usize;
        let mut j = 0usize;

        // 核心合并循环：按位置合并两个编辑序列
        while i < local_edits.len() || j < remote_edits.len() {
            let next_local = local_edits.get(i);
            let next_remote = remote_edits.get(j);

            match (next_local, next_remote) {
                (Some(local_edit), Some(remote_edit)) => {
                    if edits_overlap(local_edit, remote_edit) {
                        if edits_equivalent(local_edit, remote_edit) {
                            merged_edits.push(local_edit.clone());
                            i += 1;
                            j += 1;
                        } else {
                            conflicts.push(build_conflict_hunk(base, local_edit, remote_edit));
                            i += 1;
                            j += 1;
                        }
                    } else if local_edit.start < remote_edit.start {
                        merged_edits.push(local_edit.clone());
                        i += 1;
                    } else {
                        merged_edits.push(remote_edit.clone());
                        j += 1;
                    }
                }
                (Some(local_edit), None) => {
                    merged_edits.push(local_edit.clone());
                    i += 1;
                }
                (None, Some(remote_edit)) => {
                    merged_edits.push(remote_edit.clone());
                    j += 1;
                }
                (None, None) => break,
            }
        }

        if !conflicts.is_empty() {
            return MergeResult::Conflict {
                base: base.to_string(),
                local: local.to_string(),
                remote: remote.to_string(),
                conflicts,
            };
        }

        let merged = apply_edits(base, &merged_edits);
        MergeResult::Success(merged)
    }
}

fn build_conflict_hunk(base: &str, local: &Edit, remote: &Edit) -> ConflictHunk {
    let start_line = char_index_to_line(base, std::cmp::min(local.start, remote.start));
    let end_line = char_index_to_line(base, std::cmp::max(local.end, remote.end));
    let length = end_line.saturating_sub(start_line).saturating_add(1);

    ConflictHunk {
        start_line,
        length,
        local_lines: to_lines(&local.replacement),
        remote_lines: to_lines(&remote.replacement),
    }
}

fn char_index_to_line(s: &str, char_index: usize) -> usize {
    let mut line = 0usize;
    let mut count = 0usize;
    for ch in s.chars() {
        if count >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
        }
        count += 1;
    }
    line
}

fn to_lines(s: &str) -> Vec<String> {
    if s.is_empty() {
        return Vec::new();
    }
    s.lines().map(|line| line.to_string()).collect()
}