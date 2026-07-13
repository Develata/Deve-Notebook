// crates/core/src/source_control/commit_diff.rs
//! # 提交间差异计算 (Commit Diff)
//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 05_diff_logic#source-control-runtime
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
use crate::source_control::commit_diff_error::{CommitDiffError, CommitDiffResult};
use crate::source_control::commit_diff_paths;
use crate::source_control::commits::COMMITS_TABLE;
use crate::source_control::types::{
    ChangeStatus, CommitFileDiff, CommitFileDiffSummary, CommitFileDiffTarget,
};
use crate::state::reconstruct_content;
use anyhow::Result;
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
    compare_commits_checked(db, commit_a_id, commit_b_id).map_err(Into::into)
}

/// Build body-free summaries for browser history without retaining every file body.
pub fn compare_commit_summaries(
    db: &Database,
    commit_a_id: Option<&str>,
    commit_b_id: &str,
) -> Result<Vec<CommitFileDiffSummary>> {
    let seq_a = resolve_seq(db, commit_a_id)?;
    let seq_b = load_commit(db, commit_b_id)?.ledger_seq;
    let context = compare_context(db, seq_a, seq_b)?;
    let mut summaries = Vec::new();
    for doc_id in &context.affected_docs {
        if let Some(summary) = project_one(db, &context, *doc_id)?.map(|diff| summary_for(&diff)) {
            summaries.push(summary);
        }
    }
    summaries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(summaries)
}

/// Reconstruct exactly one summary target and reject stale/path-only substitutions.
pub fn compare_commit_file_exact(
    db: &Database,
    commit_a_id: Option<&str>,
    commit_b_id: &str,
    target: &CommitFileDiffTarget,
) -> Result<CommitFileDiff> {
    let seq_a = resolve_seq(db, commit_a_id)?;
    let seq_b = load_commit(db, commit_b_id)?.ledger_seq;
    let context = compare_context(db, seq_a, seq_b)?;
    if !context.affected_docs.contains(&target.doc_id) {
        return Err(CommitDiffError::TargetMismatch.into());
    }
    let diff = project_one(db, &context, target.doc_id)?.ok_or(CommitDiffError::TargetMismatch)?;
    if (CommitFileDiffTarget {
        doc_id: target.doc_id,
        path: diff.path.clone(),
        previous_path: diff.previous_path.clone(),
        status: diff.status,
    }) != *target
    {
        return Err(CommitDiffError::TargetMismatch.into());
    }
    Ok(diff)
}

pub(crate) fn compare_commits_checked(
    db: &Database,
    commit_a_id: Option<&str>,
    commit_b_id: &str,
) -> CommitDiffResult<Vec<CommitFileDiff>> {
    let seq_a = resolve_seq(db, commit_a_id)?;
    let commit_b = load_commit(db, commit_b_id)?;
    compare_seq_range_checked(db, seq_a, commit_b.ledger_seq)
}

pub(crate) fn compare_seq_range_checked(
    db: &Database,
    seq_a: u64,
    seq_b: u64,
) -> CommitDiffResult<Vec<CommitFileDiff>> {
    let context = compare_context(db, seq_a, seq_b)?;
    let mut diffs = Vec::new();
    for doc_id in &context.affected_docs {
        if let Some(diff) = project_one(db, &context, *doc_id)? {
            diffs.push(diff);
        }
    }
    diffs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(diffs)
}

struct CompareContext {
    seq_a: u64,
    seq_b: u64,
    path_map_a: HashMap<DocId, String>,
    path_map_b: HashMap<DocId, String>,
    affected_docs: HashSet<DocId>,
}

fn compare_context(db: &Database, seq_a: u64, seq_b: u64) -> CommitDiffResult<CompareContext> {
    if seq_a >= seq_b {
        return Err(CommitDiffError::InvalidOrder { seq_a, seq_b });
    }
    let range_start = seq_a + 1;
    let range_end = seq_b + 1;
    let ops_range =
        crate::ledger::range::get_ops_in_range(db, range_start, range_end).map_err(|err| {
            CommitDiffError::LedgerRange {
                start: range_start,
                end: range_end,
                message: err.to_string(),
            }
        })?;
    let path_map_a = commit_diff_paths::doc_paths_at_seq(db, seq_a)?;
    let path_map_b = commit_diff_paths::doc_paths_at_seq(db, seq_b)?;
    let affected_docs = collect_affected_docs(&ops_range, &path_map_a, &path_map_b);
    Ok(CompareContext {
        seq_a,
        seq_b,
        path_map_a,
        path_map_b,
        affected_docs,
    })
}

fn project_one(
    db: &Database,
    context: &CompareContext,
    doc_id: DocId,
) -> CommitDiffResult<Option<CommitFileDiff>> {
    let old_path = context.path_map_a.get(&doc_id).cloned();
    let new_path = context.path_map_b.get(&doc_id).cloned();
    let old_content = reconstruct_at_seq(db, doc_id, context.seq_a)?;
    let new_content = reconstruct_at_seq(db, doc_id, context.seq_b)?;
    if old_content == new_content && old_path == new_path {
        return Ok(None);
    }
    let Some(path) = new_path.clone().or(old_path.clone()) else {
        return Err(CommitDiffError::LostProjectedPath {
            doc_id,
            seq_a: context.seq_a,
            seq_b: context.seq_b,
        });
    };
    Ok(Some(CommitFileDiff {
        doc_id: Some(doc_id),
        status: detect_status(
            old_path.as_deref(),
            new_path.as_deref(),
            &old_content,
            &new_content,
        ),
        path,
        previous_path: (old_path != new_path).then_some(old_path).flatten(),
        old_content,
        new_content,
    }))
}

fn summary_for(diff: &CommitFileDiff) -> CommitFileDiffSummary {
    let doc_id = diff.doc_id.expect("commit diff projection is doc-backed");
    let target = CommitFileDiffTarget {
        doc_id,
        path: diff.path.clone(),
        previous_path: diff.previous_path.clone(),
        status: diff.status,
    };
    CommitFileDiffSummary {
        doc_id,
        path: diff.path.clone(),
        previous_path: diff.previous_path.clone(),
        status: diff.status,
        target,
    }
}

pub(crate) fn compare_seq_range(
    db: &Database,
    seq_a: u64,
    seq_b: u64,
) -> Result<Vec<CommitFileDiff>> {
    compare_seq_range_checked(db, seq_a, seq_b).map_err(Into::into)
}

pub(crate) fn projection_files_at_commit(
    db: &Database,
    commit_id: &str,
) -> CommitDiffResult<Vec<CommitFileDiff>> {
    let commit = load_commit(db, commit_id)?;
    let path_map = commit_diff_paths::doc_paths_at_seq(db, commit.ledger_seq)?;
    let mut files = Vec::with_capacity(path_map.len());
    for (doc_id, path) in path_map {
        let content = reconstruct_at_seq(db, doc_id, commit.ledger_seq)?;
        files.push(CommitFileDiff {
            doc_id: Some(doc_id),
            path,
            status: ChangeStatus::Added,
            previous_path: None,
            old_content: String::new(),
            new_content: content,
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
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
fn resolve_seq(db: &Database, commit_id: Option<&str>) -> CommitDiffResult<u64> {
    match commit_id {
        None => Ok(0),
        Some(id) => Ok(load_commit(db, id)?.ledger_seq),
    }
}

/// 从数据库加载单个 CommitInfo
fn load_commit(
    db: &Database,
    commit_id: &str,
) -> CommitDiffResult<crate::source_control::CommitInfo> {
    let read_txn = db
        .begin_read()
        .map_err(|err| CommitDiffError::CommitTable {
            action: "read",
            message: err.to_string(),
        })?;
    let table = read_txn
        .open_table(COMMITS_TABLE)
        .map_err(|err| CommitDiffError::CommitTable {
            action: "open",
            message: err.to_string(),
        })?;
    let json = table
        .get(commit_id)
        .map_err(|err| CommitDiffError::CommitLoad {
            commit_id: commit_id.to_string(),
            message: err.to_string(),
        })?
        .ok_or_else(|| CommitDiffError::CommitNotFound {
            commit_id: commit_id.to_string(),
        })?;
    serde_json::from_str(json.value()).map_err(|err| CommitDiffError::CommitDecode {
        commit_id: commit_id.to_string(),
        message: err.to_string(),
    })
}

/// 重建指定文档在给定序列号时刻的内容
///
/// 取该 doc_id 所有 seq <= max_seq 的 Op，按序重建
fn reconstruct_at_seq(db: &Database, doc_id: DocId, max_seq: u64) -> CommitDiffResult<String> {
    let all_ops = crate::ledger::ops::get_ops_from_db(db, doc_id).map_err(|err| {
        CommitDiffError::ContentLoad {
            doc_id,
            max_seq,
            message: err.to_string(),
        }
    })?;
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
