// crates/core/src/source_control/pending_fs.rs
//! # 待确认文件变更管理 (Pending FS Ops)
//!
//! 存储 Watcher 检测到但用户尚未确认的文件系统变更。
//! 这是 Git-like 三阶段工作流的 Working Directory 层。
//!
//! **不变量**: pending_fs_ops 中的条目永远不会自动进入 Ledger，
//! 必须经过用户显式 Stage → Commit 才会生成 Op。
//!
//! **存储结构**:
//! - Table: `pending_fs_ops` (path -> PendingFsEntry 序列化字节)

use crate::ledger::schema::PENDING_FS_OPS;
use crate::source_control::ChangeStatus;
use anyhow::Result;
use redb::{Database, ReadableTable};
use serde::{Deserialize, Serialize};

/// 待确认的文件变更条目
///
/// **Pre-condition**: path 已经过 `to_forward_slash` 规范化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingFsEntry {
    /// 相对路径 (forward-slash)
    pub path: String,
    /// 变更类型
    pub change_type: ChangeStatus,
    /// 内容 SHA-256 哈希 (16 字节前缀，hex 编码)
    pub content_hash: String,
    /// 检测时间戳 (毫秒)
    pub detected_at: i64,
}

/// 初始化 pending_fs_ops 表
pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(PENDING_FS_OPS)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 插入或更新一条待确认变更
///
/// **Invariant**: 同一 path 只保留最新的 entry（幂等写入）。
pub fn upsert(db: &Database, entry: &PendingFsEntry) -> Result<()> {
    let bytes = serde_json::to_vec(entry)?;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(PENDING_FS_OPS)?;
        table.insert(entry.path.as_str(), bytes.as_slice())?;
    }
    write_txn.commit()?;
    tracing::debug!(
        "Pending FS upsert: {} ({:?})",
        entry.path,
        entry.change_type
    );
    Ok(())
}

/// 获取所有待确认变更
pub fn list_all(db: &Database) -> Result<Vec<PendingFsEntry>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PENDING_FS_OPS)?;
    let mut entries = Vec::new();
    for item in table.iter()? {
        let (_key, value) = item?;
        let entry: PendingFsEntry = serde_json::from_slice(value.value())?;
        entries.push(entry);
    }
    Ok(entries)
}

/// 获取单条待确认变更
pub fn get(db: &Database, path: &str) -> Result<Option<PendingFsEntry>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PENDING_FS_OPS)?;
    match table.get(path)? {
        Some(guard) => {
            let entry: PendingFsEntry = serde_json::from_slice(guard.value())?;
            Ok(Some(entry))
        }
        None => Ok(None),
    }
}

/// 移除单条待确认变更（Stage 或 Discard 后调用）
pub fn remove(db: &Database, path: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(PENDING_FS_OPS)?;
        table.remove(path)?;
    }
    write_txn.commit()?;
    tracing::debug!("Pending FS removed: {}", path);
    Ok(())
}

/// 清空所有待确认变更
pub fn clear(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        write_txn.delete_table(PENDING_FS_OPS)?;
        let _ = write_txn.open_table(PENDING_FS_OPS)?;
    }
    write_txn.commit()?;
    tracing::info!("Cleared all pending FS ops");
    Ok(())
}

/// 计算内容的 SHA-256 哈希前缀 (hex, 16 字节 = 32 hex chars)
///
/// 用于快速判断文件内容是否真正变化（防抖）。
pub fn content_hash(content: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
