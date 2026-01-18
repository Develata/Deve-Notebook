// crates\core\src\source_control
//! # 暂存区管理 (Staging Manager)
//!
//! 管理文件的暂存状态，持久化到数据库。
//!
//! **存储结构**:
//! - Table: `staged_files` - 存储已暂存的文件路径

use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};

/// 暂存区表定义 (路径 -> 时间戳)
pub const STAGED_TABLE: TableDefinition<&str, i64> = TableDefinition::new("staged_files");

/// 初始化暂存区表
pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(STAGED_TABLE)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 暂存指定文件
pub fn stage(db: &Database, path: &str) -> Result<()> {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(STAGED_TABLE)?;
        table.insert(path, timestamp)?;
    }
    write_txn.commit()?;
    tracing::info!("Staged file: {}", path);
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

/// 获取所有已暂存的文件
///
/// **注意**: 此操作会将所有路径加载到内存。对于大规模暂存 (>10k 文件) 可能有性能影响。
pub fn list_staged(db: &Database) -> Result<Vec<String>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(STAGED_TABLE)?;
    let mut paths = Vec::new();
    for entry in table.iter()? {
        let (key, _value): (redb::AccessGuard<&str>, redb::AccessGuard<i64>) = entry?;
        paths.push(key.value().to_string());
    }
    Ok(paths)
}

/// 清空暂存区 (提交后调用)
pub fn clear(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        // Optimization: Drop and recreate table
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
