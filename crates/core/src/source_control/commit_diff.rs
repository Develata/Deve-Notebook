// crates/core/src/source_control/commit_diff.rs
//! # 提交间差异计算 (Commit Diff)
//!
//! 对比两个提交之间的文件变更。
//!
//! **算法**:
//! 1. 查找两个 commit 的 ledger_seq 范围
//! 2. 取范围内所有 Op，按 doc_id 分组
//! 3. 对每个 doc，重建 commit_a / commit_b 时刻的内容
//! 4. 生成统一 diff
//!
//! **Invariant**: commit_a.ledger_seq < commit_b.ledger_seq (时序约束)

use crate::models::DocId;
use crate::source_control::commits::COMMITS_TABLE;
use crate::source_control::types::{ChangeStatus, CommitFileDiff};
use crate::state::reconstruct_content;
use anyhow::Result;
use redb::Database;
use std::collections::HashMap;

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

    // 收集范围内所有受影响的 doc_id
    let ops_range = crate::ledger::range::get_ops_in_range(db, seq_a + 1, seq_b + 1)?;
    let mut affected_docs: HashMap<DocId, String> = HashMap::new();

    // 从 snapshot_paths 取 doc_id -> path 映射
    let path_map = build_path_map(db)?;

    for (_seq, entry) in &ops_range {
        affected_docs
            .entry(entry.doc_id)
            .or_insert_with(|| path_map.get(&entry.doc_id).cloned().unwrap_or_default());
    }

    // 对每个受影响的 doc，重建两个时刻的内容
    let mut diffs = Vec::new();
    for (doc_id, path) in &affected_docs {
        let old = reconstruct_at_seq(db, *doc_id, seq_a)?;
        let new = reconstruct_at_seq(db, *doc_id, seq_b)?;
        if old == new {
            continue;
        }
        let status = detect_status(&old, &new);
        diffs.push(CommitFileDiff {
            path: path.clone(),
            status,
            old_content: old,
            new_content: new,
        });
    }
    diffs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(diffs)
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

/// 构建 doc_id -> path 映射 (从 metadata + snapshot_paths)
fn build_path_map(db: &Database) -> Result<HashMap<DocId, String>> {
    let mut map = HashMap::new();
    // 优先从 docid_to_path 取
    let docs = crate::ledger::metadata::list_docs(db)?;
    for (doc_id, path) in docs {
        map.insert(doc_id, path);
    }
    // 补充 snapshot_paths (已删除的文件可能不在 docid_to_path 中)
    let snap_paths = crate::source_control::snapshot_paths::list_snapshot_paths(db)?;
    for (doc_id, path) in snap_paths {
        map.entry(doc_id).or_insert(path);
    }
    Ok(map)
}

/// 检测变更状态
fn detect_status(old: &str, new: &str) -> ChangeStatus {
    if old.is_empty() {
        ChangeStatus::Added
    } else if new.is_empty() {
        ChangeStatus::Deleted
    } else {
        ChangeStatus::Modified
    }
}
