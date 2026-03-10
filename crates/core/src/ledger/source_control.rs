// crates\core\src\ledger
//! # Source Control 集成 (RepoManager 扩展)
//!
//! 为 RepoManager 提供 Source Control 功能的便捷方法。
//!
//! **功能**:
//! - 暂存区操作 (stage/unstage)
//! - 提交管理 (create/list commits)
//! - 变更检测 (获取未提交的文件)

use crate::models::DocId;
use crate::source_control::{
    ChangeEntry, ChangeStatus, CommitInfo, changes, commits, pending_fs, staging,
};
use anyhow::Result;
use redb::Database;

/// 初始化 Source Control 相关的数据库表
pub fn init_tables(db: &Database) -> Result<()> {
    staging::init_table(db)?;
    commits::init_table(db)?;
    changes::init_table(db)?;
    pending_fs::init_table(db)?;
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
    let entries = staging::list_staged_entries(db)?;
    Ok(entries
        .into_iter()
        .map(|(path, entry)| ChangeEntry {
            path,
            renamed_from: entry.renamed_from,
            doc_id: entry.doc_id,
            status: entry.status,
            has_conflict: entry.has_conflict,
        })
        .collect())
}

/// 获取提交历史
pub fn list_commits(db: &Database, limit: u32) -> Result<Vec<CommitInfo>> {
    commits::list(db, limit)
}

/// 获取文档的已提交内容 (快照)
pub fn get_committed_content(db: &Database, doc_id: DocId) -> Result<Option<String>> {
    changes::get_committed_content(db, doc_id)
}

/// 检测单个文档的变更状态
pub fn detect_change(committed: Option<&str>, current: Option<&str>) -> Option<ChangeStatus> {
    changes::detect_doc_change(committed, current)
}
