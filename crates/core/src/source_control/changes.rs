// crates\core\src\source_control
//! # 变更检测模块 (Change Detection)
//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 05_diff_logic#source-control-runtime
//!
//! 对比当前 Ledger 状态与最后提交快照，检测文件变更。
//!
//! **核心功能**:
//! - `detect_all_changes`: 检测所有文档的变更状态
//! - `get_committed_content`: 获取文档的最后提交内容

use crate::models::DocId;
use crate::source_control::ChangeStatus;
use anyhow::Result;
use redb::{Database, TableDefinition, WriteTransaction};

/// 快照表定义 (doc_id -> content)
/// 存储每个文档最后一次提交时的内容
pub const SNAPSHOTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("commit_snapshots");

/// 初始化快照表
pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(SNAPSHOTS_TABLE)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 保存文档快照 (提交时调用)
///
/// **参数**:
/// - `doc_id`: 文档 ID
/// - `content`: 文档当前内容
pub fn save_snapshot(db: &Database, doc_id: DocId, content: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    save_snapshot_in_txn(&write_txn, doc_id, content)?;
    write_txn.commit()?;
    tracing::debug!("Saved snapshot for doc: {}", doc_id);
    Ok(())
}

pub(crate) fn save_snapshot_in_txn(
    write_txn: &WriteTransaction,
    doc_id: DocId,
    content: &str,
) -> Result<()> {
    let doc_id = doc_id.to_string();
    write_txn
        .open_table(SNAPSHOTS_TABLE)?
        .insert(doc_id.as_str(), content)?;
    Ok(())
}

/// 获取文档的最后提交内容
///
/// **返回**: `Some(content)` 如果有快照，`None` 如果是新文档
pub fn get_committed_content(db: &Database, doc_id: DocId) -> Result<Option<String>> {
    let doc_id_str = doc_id.to_string();
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(SNAPSHOTS_TABLE)?;

    match table.get(doc_id_str.as_str())? {
        Some(guard) => Ok(Some(guard.value().to_string())),
        None => Ok(None),
    }
}

/// 检测单个文档的变更状态
///
/// **逻辑**:
/// - 无快照 + 有当前内容 = Added
/// - 有快照 + 无当前内容 = Deleted
/// - 快照 != 当前内容 = Modified
/// - 相同 = None
pub fn detect_doc_change(committed: Option<&str>, current: Option<&str>) -> Option<ChangeStatus> {
    match (committed, current) {
        // 新文档 (无快照但有当前内容)
        (None, Some(cur)) if !cur.is_empty() => Some(ChangeStatus::Added),
        // 已删除 (有快照但无当前内容)
        (Some(_), None) => Some(ChangeStatus::Deleted),
        // 已修改 (内容不同)
        (Some(old), Some(new)) if old != new => Some(ChangeStatus::Modified),
        // 无变更
        _ => None,
    }
}

/// 清空所有快照 (重置用)
pub fn clear_snapshots(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        // Optimization: Drop and recreate the table is faster than deleting row by row
        // Redb supports delete_table
        write_txn.delete_table(SNAPSHOTS_TABLE)?;
        let _ = write_txn.open_table(SNAPSHOTS_TABLE)?;
    }
    write_txn.commit()?;
    tracing::info!("Cleared all snapshots");
    Ok(())
}

/// 删除文档快照 (提交删除时调用)
pub fn remove_snapshot(db: &Database, doc_id: DocId) -> Result<()> {
    let write_txn = db.begin_write()?;
    remove_snapshot_in_txn(&write_txn, doc_id)?;
    write_txn.commit()?;
    tracing::debug!("Removed snapshot for doc: {}", doc_id);
    Ok(())
}

pub(crate) fn remove_snapshot_in_txn(write_txn: &WriteTransaction, doc_id: DocId) -> Result<()> {
    let doc_id = doc_id.to_string();
    write_txn
        .open_table(SNAPSHOTS_TABLE)?
        .remove(doc_id.as_str())?;
    Ok(())
}
