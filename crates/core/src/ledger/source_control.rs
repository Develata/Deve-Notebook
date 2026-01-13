//! # Source Control 集成 (RepoManager 扩展)
//!
//! 为 RepoManager 提供 Source Control 功能的便捷方法。
//!
//! **功能**:
//! - 暂存区操作 (stage/unstage)
//! - 提交管理 (create/list commits)
//! - 变更检测 (获取未提交的文件)

use anyhow::Result;
use redb::Database;
use crate::source_control::{commits, staging, ChangeEntry, ChangeStatus, CommitInfo};

/// 初始化 Source Control 相关的数据库表
pub fn init_tables(db: &Database) -> Result<()> {
    staging::init_table(db)?;
    commits::init_table(db)?;
    Ok(())
}

/// 暂存指定文件
pub fn stage_file(db: &Database, path: &str) -> Result<()> {
    staging::stage(db, path)
}

/// 取消暂存指定文件
pub fn unstage_file(db: &Database, path: &str) -> Result<()> {
    staging::unstage(db, path)
}

/// 获取已暂存的文件列表
pub fn list_staged(db: &Database) -> Result<Vec<ChangeEntry>> {
    let paths = staging::list_staged(db)?;
    Ok(paths.into_iter()
        .map(|path| ChangeEntry { path, status: ChangeStatus::Modified })
        .collect())
}

/// 创建提交
///
/// **流程**:
/// 1. 获取暂存区的文件数量
/// 2. 创建提交记录
/// 3. 清空暂存区
pub fn create_commit(db: &Database, message: &str) -> Result<CommitInfo> {
    let staged = staging::list_staged(db)?;
    let doc_count = staged.len() as u32;
    
    if doc_count == 0 {
        anyhow::bail!("Nothing to commit: staging area is empty");
    }
    
    let commit = commits::create(db, message, doc_count)?;
    staging::clear(db)?;
    
    Ok(commit)
}

/// 获取提交历史
pub fn list_commits(db: &Database, limit: u32) -> Result<Vec<CommitInfo>> {
    commits::list(db, limit)
}
