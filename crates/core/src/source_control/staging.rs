// crates\core\src\source_control
//! # 暂存区管理 (Staging Manager)
//!
//! 管理文件的暂存状态，持久化到数据库。
//!
//! **存储结构**:
//! - Table: `staged_files` - 存储已暂存的文件路径及其变更状态

use crate::source_control::ChangeStatus;
use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

/// 暂存区表定义 (path -> JSON bytes)
pub const STAGED_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("staged_files");

/// 暂存条目（包含变更状态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedEntry {
    pub timestamp: i64,
    pub status: ChangeStatus,
}

/// 初始化暂存区表
pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(STAGED_TABLE)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 暂存指定文件（默认 Modified）
pub fn stage(db: &Database, path: &str) -> Result<()> {
    stage_with_status(db, path, ChangeStatus::Modified)
}

/// 暂存指定文件（带变更状态）
pub fn stage_with_status(db: &Database, path: &str, status: ChangeStatus) -> Result<()> {
    let entry = StagedEntry {
        timestamp: chrono::Utc::now().timestamp_millis(),
        status,
    };
    let bytes = serde_json::to_vec(&entry)?;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(STAGED_TABLE)?;
        table.insert(path, bytes.as_slice())?;
    }
    write_txn.commit()?;
    tracing::info!("Staged file: {} ({:?})", path, entry.status);
    Ok(())
}

/// 取消暂存指定文件
pub fn unstage(db: &Database, path: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(STAGED_TABLE)?;
        table.remove(path)?;
    }
    write_txn.commit()?;
    tracing::info!("Unstaged file: {}", path);
    Ok(())
}

/// 获取所有已暂存的文件 (路径列表)
pub fn list_staged(db: &Database) -> Result<Vec<String>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(STAGED_TABLE)?;
    let mut paths = Vec::new();
    for entry in table.iter()? {
        let (key, _value) = entry?;
        paths.push(key.value().to_string());
    }
    Ok(paths)
}

/// 获取所有已暂存的文件（带变更状态）
pub fn list_staged_with_status(db: &Database) -> Result<Vec<(String, ChangeStatus)>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(STAGED_TABLE)?;
    let mut entries = Vec::new();
    for item in table.iter()? {
        let (key, value) = item?;
        let path = key.value().to_string();
        // 兼容旧版数据（无 JSON，默认 Modified）
        let status = serde_json::from_slice::<StagedEntry>(value.value())
            .map(|e| e.status)
            .unwrap_or(ChangeStatus::Modified);
        entries.push((path, status));
    }
    Ok(entries)
}

/// 清空暂存区 (提交后调用)
pub fn clear(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        write_txn.delete_table(STAGED_TABLE)?;
        let _ = write_txn.open_table(STAGED_TABLE)?;
    }
    write_txn.commit()?;
    tracing::info!("Cleared staging area");
    Ok(())
}

/// 检查文件是否已暂存
pub fn is_staged(db: &Database, path: &str) -> Result<bool> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(STAGED_TABLE)?;
    Ok(table.get(path)?.is_some())
}
