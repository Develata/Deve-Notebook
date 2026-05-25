// crates/core/src/source_control/conflict.rs
//! # 冲突检测模块 (Conflict Detection)
//! plan_ref:
//!   - 03_storage#watcher-contract
//!   - 05_diff_logic#source-control-runtime
//!
//! 当文件同时在文件系统和 Ledger 中存在未提交变更时，标记为冲突。
//!
//! **不变量**: 冲突仅在 pending_fs_ops 条目上标记（`has_conflict` 字段），
//! 不引入额外存储表。
//!
//! **算法复杂度**: O(1) per fs event — 仅需两次哈希比较。

use crate::ledger::ops;
use crate::source_control::changes;
use crate::source_control::pending_fs;
use anyhow::Result;
use redb::Database;

/// 检测指定路径是否存在 FS vs Ledger 冲突
///
/// **定义**: 冲突 ⟺ FS 偏离已提交快照 ∧ Ledger 也偏离已提交快照。
///
/// **Pre-condition**: `pending_hash` 为 Watcher 计算的文件内容哈希。
/// **Post-condition**: 返回 true 表示存在冲突，需用户介入。
pub fn check_conflict(
    db: &Database,
    doc_id: crate::models::DocId,
    pending_hash: &str,
) -> Result<bool> {
    // 获取已提交快照内容
    let committed_content = changes::get_committed_content(db, doc_id)?;
    let committed_hash = match &committed_content {
        Some(content) => pending_fs::content_hash(content),
        None => String::new(), // 新文档，无快照
    };

    // FS 是否偏离已提交版本
    let fs_diverged = pending_hash != committed_hash;
    if !fs_diverged {
        return Ok(false); // FS 未变更，无冲突
    }

    // 获取 Ledger 当前内容并计算哈希
    let ledger_content = get_ledger_content(db, doc_id)?;
    let ledger_hash = match &ledger_content {
        Some(content) => pending_fs::content_hash(content),
        None => String::new(),
    };

    // Ledger 是否也偏离已提交版本
    let ledger_diverged = ledger_hash != committed_hash;

    Ok(fs_diverged && ledger_diverged)
}

/// 从 Ledger 重建文档当前内容。
fn get_ledger_content(db: &Database, doc_id: crate::models::DocId) -> Result<Option<String>> {
    let entries = ops::get_ops_from_db(db, doc_id)?;
    if entries.is_empty() {
        return Ok(None);
    }
    let facts: Vec<_> = entries.into_iter().map(|(_, entry)| entry).collect();
    Ok(Some(crate::state::reconstruct_content(&facts)))
}
