// crates\core\src\ledger
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/watcher#watcher-contract
//!   - 03_storage/index#repo-runtime-layout
//!
//! # Source Control 集成 (RepoManager 扩展)
//!
//! 为 RepoManager 提供 Source Control 功能的便捷方法。
//!
//! **功能**:
//! - 暂存区操作 (stage/unstage)
//! - 提交管理 (create/list commits)
//! - 变更检测 (获取未提交的文件)

use crate::ledger::schema::{PENDING_FS_DOC_INDEX, PENDING_FS_OPS, STAGED_DOC_INDEX};
use crate::models::DocId;
use crate::source_control::{
    ChangeDomain, ChangeEntry, ChangeStatus, CommitInfo, changes, commits, pending_fs, staging,
};
use anyhow::Result;
use redb::Database;

pub(crate) struct StagedChangeProjection {
    pub change: ChangeEntry,
    pub resolved_conflict: bool,
}

/// 初始化 Source Control 相关的数据库表
pub fn init_tables(db: &Database) -> Result<()> {
    staging::init_table(db)?;
    commits::init_table(db)?;
    changes::init_table(db)?;
    pending_fs::init_table(db)?;
    Ok(())
}

/// Invariants:
/// - runtime local-repo access must not create missing source-control tables
/// - missing source-control tables are treated as catalog corruption until explicit repair
pub fn validate_tables(db: &Database) -> Result<()> {
    let read_txn = db.begin_read()?;
    let _ = read_txn.open_table(staging::STAGED_TABLE)?;
    let _ = read_txn.open_multimap_table(STAGED_DOC_INDEX)?;
    let _ = read_txn.open_table(commits::COMMITS_TABLE)?;
    let _ = read_txn.open_table(commits::COMMITS_ORDER_TABLE)?;
    let _ = read_txn.open_table(changes::SNAPSHOTS_TABLE)?;
    let _ = read_txn.open_table(PENDING_FS_OPS)?;
    let _ = read_txn.open_multimap_table(PENDING_FS_DOC_INDEX)?;
    Ok(())
}

pub fn stage_pending_entry(db: &Database, entry: &pending_fs::PendingFsEntry) -> Result<()> {
    staging::stage_pending_entry(db, entry)
}

pub fn take_staged_entry(db: &Database, path: &str) -> Result<Option<staging::StagedEntry>> {
    staging::take_staged(db, path)
}

/// 获取已暂存的文件列表 (含正确的变更状态)
pub fn list_staged(db: &Database) -> Result<Vec<ChangeEntry>> {
    Ok(list_staged_projection(db)?
        .into_iter()
        .map(|entry| entry.change)
        .collect())
}

pub(crate) fn list_staged_projection(db: &Database) -> Result<Vec<StagedChangeProjection>> {
    let entries = staging::list_staged_entries(db)?;
    Ok(entries
        .into_iter()
        .map(|(path, entry)| StagedChangeProjection {
            resolved_conflict: entry.resolved_conflict,
            change: ChangeEntry {
                path,
                renamed_from: entry.renamed_from,
                doc_id: entry.doc_id,
                status: entry.status,
                has_conflict: entry.has_conflict,
                domain: ChangeDomain::Staged,
                base_seq: None,
                target_seq: None,
            },
        })
        .collect())
}

/// 获取提交历史
pub fn list_commits(db: &Database, limit: u32) -> Result<Vec<CommitInfo>> {
    commits::list(db, limit)
}

pub fn repair_missing_commit_order_table(db: &Database) -> Result<()> {
    commits::repair_missing_order_table(db)
}

/// 获取文档的已提交内容 (快照)
pub fn get_committed_content(db: &Database, doc_id: DocId) -> Result<Option<String>> {
    changes::get_committed_content(db, doc_id)
}

/// 检测单个文档的变更状态
pub fn detect_change(committed: Option<&str>, current: Option<&str>) -> Option<ChangeStatus> {
    changes::detect_doc_change(committed, current)
}
