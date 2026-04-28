// crates/core/src/source_control/commit_diff.rs
//! # 提交间差异计算 (Commit Diff)
//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 07_diff_logic#source-control-runtime
//!
//! 对比两个提交之间的文件变更。
//!
//! **算法**:
//! 1. 查找两个 commit 的 ledger_seq 范围
//! 2. 回放 `Structure Facts`，得到 `seq_a` / `seq_b` 时刻的 doc path 投影
//! 3. 仅对“内容变化或路径变化”的 doc 生成差异
//! 4. 输出结构感知的统一 diff
//!
//! **Invariant**: commit_a.ledger_seq < commit_b.ledger_seq (时序约束)

use crate::models::DocId;
use crate::source_control::commit_diff_paths;
use crate::source_control::commits::COMMITS_TABLE;
use crate::source_control::types::{ChangeStatus, CommitFileDiff};
use crate::state::reconstruct_content;
use anyhow::{Result, anyhow};
use redb::Database;
use std::collections::{HashMap, HashSet};

/// 对比两个提交之间的文件变更
///
/// **参数**:
/// - `commit_a_id`: 较早的提交 (None = 空状态，用于查看首次提交的全量变更)
/// - `commit_b_id`: 较新的提交
///
/// **返回**: 变更文件列表，每个包含旧/新内容
pub fn compare_commits(
    db: &Database,
    commit_a_id: Option<&str>,
    commit_b_id: &str,
) -> Result<Vec<CommitFileDiff>> {
    let seq_a = resolve_seq(db, commit_a_id)?;
    let commit_b = load_commit(db, commit_b_id)?;
    let seq_b = commit_b.ledger_seq;

    let ops_range = crate::ledger::range::get_ops_in_range(db, seq_a + 1, seq_b + 1)?;
    let path_map_a = commit_diff_paths::doc_paths_at_seq(db, seq_a)?;
    let path_map_b = commit_diff_paths::doc_paths_at_seq(db, seq_b)?;
    let mut affected_docs = collect_affected_docs(&ops_range, &path_map_a, &path_map_b);

    for (_seq, entry) in &ops_range {
        let Some(doc_id) = entry.doc_id else {
            continue;
        };
        affected_docs.insert(doc_id);
    }

    let mut diffs = Vec::new();
    for doc_id in affected_docs.drain() {
        let old_path = path_map_a.get(&doc_id).cloned();
        let new_path = path_map_b.get(&doc_id).cloned();
        let old_content = reconstruct_at_seq(db, doc_id, seq_a)?;
        let new_content = reconstruct_at_seq(db, doc_id, seq_b)?;
        if old_content == new_content && old_path == new_path {
            continue;
        }
        let Some(path) = new_path.clone().or(old_path.clone()) else {
            return Err(anyhow!(
                "Commit diff lost projected path for doc {} between seq {} and {}",
                doc_id,
                seq_a,
                seq_b
            ));
        };
        let status = detect_status(
            old_path.as_deref(),
            new_path.as_deref(),
            &old_content,
            &new_content,
        );
        diffs.push(CommitFileDiff {
            path,
            status,
            previous_path: (old_path != new_path).then_some(old_path).flatten(),
            old_content,
            new_content,
        });
    }
    diffs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(diffs)
}

fn collect_affected_docs(
    ops_range: &[(u64, crate::models::LedgerEntry)],
    path_map_a: &HashMap<DocId, String>,
    path_map_b: &HashMap<DocId, String>,
) -> HashSet<DocId> {
    let mut docs = HashSet::new();
    for (_, entry) in ops_range {
        if let Some(doc_id) = entry.doc_id {
            docs.insert(doc_id);
        }
    }
    for doc_id in path_map_a.keys().chain(path_map_b.keys()) {
        if path_map_a.get(doc_id) != path_map_b.get(doc_id) {
            docs.insert(*doc_id);
        }
    }
    docs
}

/// 解析 commit_id 对应的 ledger_seq，None 表示空状态 (seq=0)
fn resolve_seq(db: &Database, commit_id: Option<&str>) -> Result<u64> {
    match commit_id {
        None => Ok(0),
        Some(id) => Ok(load_commit(db, id)?.ledger_seq),
    }
}

/// 从数据库加载单个 CommitInfo
fn load_commit(db: &Database, commit_id: &str) -> Result<crate::source_control::CommitInfo> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(COMMITS_TABLE)?;
    let json = table
        .get(commit_id)?
        .ok_or_else(|| anyhow::anyhow!("Commit not found: {}", commit_id))?;
    Ok(serde_json::from_str(json.value())?)
}

/// 重建指定文档在给定序列号时刻的内容
///
/// 取该 doc_id 所有 seq <= max_seq 的 Op，按序重建
fn reconstruct_at_seq(db: &Database, doc_id: DocId, max_seq: u64) -> Result<String> {
    let all_ops = crate::ledger::ops::get_ops_from_db(db, doc_id)?;
    let filtered: Vec<_> = all_ops
        .into_iter()
        .filter(|(seq, _)| *seq <= max_seq)
        .map(|(_, entry)| entry)
        .collect();
    Ok(reconstruct_content(&filtered))
}

/// 检测变更状态
fn detect_status(
    old_path: Option<&str>,
    new_path: Option<&str>,
    old_content: &str,
    new_content: &str,
) -> ChangeStatus {
    if old_path.is_none() {
        ChangeStatus::Added
    } else if new_path.is_none() {
        ChangeStatus::Deleted
    } else if old_path != new_path {
        ChangeStatus::Renamed
    } else if old_content != new_content {
        ChangeStatus::Modified
    } else {
        ChangeStatus::Renamed
    }
}
