// crates\core\src\ledger
//! # Source Control 集成 (RepoManager 扩展)
//!
//! 为 RepoManager 提供 Source Control 功能的便捷方法。
//!
//! **功能**:
//! - 暂存区操作 (stage/unstage)
//! - 提交管理 (create/list commits)
//! - 变更检测 (获取未提交的文件)

use crate::ledger::range;
use crate::models::DocId;
use crate::source_control::{ChangeEntry, ChangeStatus, CommitInfo, changes, commits, staging};
use anyhow::Result;
use redb::Database;

/// 初始化 Source Control 相关的数据库表
pub fn init_tables(db: &Database) -> Result<()> {
    staging::init_table(db)?;
    commits::init_table(db)?;
    changes::init_table(db)?;
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
    Ok(paths
        .into_iter()
        .map(|path| ChangeEntry {
            path,
            status: ChangeStatus::Modified,
        })
        .collect())
}

/// 创建提交
///
/// **流程**:
/// 1. 获取暂存区的文件
/// 2. 保存每个文件的内容快照
/// 3. 创建提交记录
/// 4. 清空暂存区
pub fn create_commit(
    db: &Database,
    message: &str,
    get_content: impl Fn(&str) -> Option<(DocId, String)>,
) -> Result<CommitInfo> {
    let staged = staging::list_staged(db)?;
    let doc_count = staged.len() as u32;

    if doc_count == 0 {
        anyhow::bail!("Nothing to commit: staging area is empty");
    }

    // 保存每个暂存文件的快照
    for path in &staged {
        if let Some((doc_id, content)) = get_content(path) {
            changes::save_snapshot(db, doc_id, &content)?;
        }
    }

    let ledger_seq = range::get_max_seq(db)?;
    let commit = commits::create(db, message, doc_count, ledger_seq)?;
    staging::clear(db)?;

    Ok(commit)
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
