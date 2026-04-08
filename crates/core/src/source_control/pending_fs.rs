// crates/core/src/source_control/pending_fs.rs
//! plan_ref:
//!   - 04_storage.md §Inode/DocId Mapping & Watcher Service
//!   - 04_storage.md §Watcher Architecture
//!
//! # 待确认文件变更管理 (Pending FS Ops)
//!
//! 存储 Watcher 检测到但用户尚未确认的文件系统变更。
//! 这是 Git-like 三阶段工作流的 Working Directory 层。
//!
//! **不变量**: pending_fs_ops 中的条目永远不会自动进入 Ledger，
//! 必须经过用户显式 Stage → Commit 才会生成 Op。
//!
//! **Idempotency Invariant (幂等性不变量)**:
//! 重复信号触发同一 `(path, status, hash, ...)` 的 upsert **MUST** 产生
//! 字节相同的 side table 行。`detected_at` 仅在语义字段变化时更新；
//! 若所有语义字段与已存在条目相等，`upsert` **MUST** 跳过写入，
//! 保持原行（含原 `detected_at`）不变。详见 plan §Watcher Architecture。
//!
//! **存储结构**:
//! - Table: `pending_fs_ops` (path -> PendingFsEntry 序列化字节)

#[path = "pending_fs_index.rs"]
mod index;
#[path = "pending_fs_query.rs"]
mod query;
#[path = "pending_fs_target.rs"]
mod target;

#[cfg(test)]
#[path = "pending_fs_idempotency_test.rs"]
mod idempotency_test;

use crate::ledger::schema::PENDING_FS_OPS;
use crate::models::DocId;
use crate::source_control::ChangeStatus;
use anyhow::Result;
pub use query::{get, list_all, list_for_doc};
use redb::{Database, ReadableTable};
use serde::{Deserialize, Serialize};
pub use target::{get_for_target, take_for_target};

/// 待确认的文件变更条目
///
/// **Pre-condition**: path 已经过 `to_forward_slash` 规范化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingFsEntry {
    /// 相对路径 (forward-slash)
    pub path: String,
    /// 若该条目由 rename/move 产生，则记录旧路径
    #[serde(default)]
    pub renamed_from: Option<String>,
    /// 与当前候选绑定的稳定文档标识；纯新增文件允许为空
    #[serde(default)]
    pub doc_id: Option<DocId>,
    /// 变更类型
    pub change_type: ChangeStatus,
    /// 内容 SHA-256 哈希 (16 字节前缀，hex 编码)
    pub content_hash: String,
    /// 检测时间戳 (毫秒)
    pub detected_at: i64,
    /// 是否存在冲突 (FS 与 Ledger 均有未提交变更)
    #[serde(default)]
    pub has_conflict: bool,
}

/// 初始化 pending_fs_ops 表
pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(PENDING_FS_OPS)?;
        index::init_table(&write_txn)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 判断两条 entry 的语义字段是否完全相等（忽略 `detected_at`）
fn semantic_eq(a: &PendingFsEntry, b: &PendingFsEntry) -> bool {
    a.path == b.path
        && a.renamed_from == b.renamed_from
        && a.doc_id == b.doc_id
        && a.change_type == b.change_type
        && a.content_hash == b.content_hash
        && a.has_conflict == b.has_conflict
}

/// 插入或更新一条待确认变更
///
/// **Invariant**: 同一 path 只保留最新的 entry（幂等写入）。
/// 若新 entry 与已有行的语义字段完全相等，**MUST** 跳过写入，
/// 保持原行（含 `detected_at`）字节不变，以满足 plan §Watcher Architecture
/// 的重复信号幂等性要求。
pub fn upsert(db: &Database, entry: &PendingFsEntry) -> Result<()> {
    let write_txn = db.begin_write()?;
    let mut skipped = false;
    {
        let mut table = write_txn.open_table(PENDING_FS_OPS)?;
        let previous = table
            .get(entry.path.as_str())?
            .map(|guard| serde_json::from_slice::<PendingFsEntry>(guard.value()))
            .transpose()?;
        if let Some(prev) = previous.as_ref()
            && semantic_eq(prev, entry)
        {
            // Byte-stable idempotency: leave existing row untouched.
            skipped = true;
        }
        if !skipped {
            let bytes = serde_json::to_vec(entry)?;
            index::replace(
                &write_txn,
                previous.as_ref().and_then(|item| item.doc_id),
                entry.doc_id,
                &entry.path,
            )?;
            table.insert(entry.path.as_str(), bytes.as_slice())?;
        }
    }
    write_txn.commit()?;
    if skipped {
        tracing::trace!(
            "Pending FS upsert (idempotent skip): {} ({:?})",
            entry.path,
            entry.change_type
        );
    } else {
        tracing::debug!(
            "Pending FS upsert: {} ({:?})",
            entry.path,
            entry.change_type
        );
    }
    Ok(())
}

/// 移除单条待确认变更（Stage 或 Discard 后调用）
pub fn remove(db: &Database, path: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(PENDING_FS_OPS)?;
        let previous = table
            .get(path)?
            .map(|guard| serde_json::from_slice::<PendingFsEntry>(guard.value()))
            .transpose()?;
        index::remove(&write_txn, previous.and_then(|entry| entry.doc_id), path)?;
        table.remove(path)?;
    }
    write_txn.commit()?;
    tracing::debug!("Pending FS removed: {}", path);
    Ok(())
}

/// 按稳定 `DocId` 原子移动单条 pending 记录。
///
/// Invariants:
/// - 若 `old_path` 对应条目不存在或 `doc_id` 不匹配，则不做任何修改。
/// - 若 `new_path` 已存在其他 pending 条目，则必须 fail-closed。
pub fn move_for_doc(db: &Database, doc_id: DocId, old_path: &str, new_path: &str) -> Result<bool> {
    let write_txn = db.begin_write()?;
    let moved = {
        let mut table = write_txn.open_table(PENDING_FS_OPS)?;
        let previous = table
            .get(old_path)?
            .map(|guard| serde_json::from_slice::<PendingFsEntry>(guard.value()))
            .transpose()?;
        if let Some(entry) = previous.filter(|entry| entry.doc_id == Some(doc_id)) {
            if table.get(new_path)?.is_some() {
                anyhow::bail!("Pending FS target already exists: {}", new_path);
            }
            let moved = PendingFsEntry {
                path: new_path.to_string(),
                ..entry
            };
            let bytes = serde_json::to_vec(&moved)?;
            index::remove(&write_txn, moved.doc_id, old_path)?;
            table.remove(old_path)?;
            index::replace(&write_txn, None, moved.doc_id, new_path)?;
            table.insert(new_path, bytes.as_slice())?;
            true
        } else {
            false
        }
    };
    write_txn.commit()?;
    if moved {
        tracing::debug!("Pending FS moved: {} -> {}", old_path, new_path);
    }
    Ok(moved)
}

/// 原子移除某个路径前缀下的所有 pending 记录。
///
/// Invariant:
/// - 同一事务内同时更新 `pending_fs_ops` 与 doc 索引。
pub fn remove_subtree(db: &Database, prefix: &str) -> Result<usize> {
    let write_txn = db.begin_write()?;
    let removed = {
        let mut table = write_txn.open_table(PENDING_FS_OPS)?;
        let mut to_remove = Vec::new();
        let prefix_slash = format!("{prefix}/");
        for item in table.iter()? {
            let (path_guard, value_guard) = item?;
            let path = path_guard.value().to_string();
            if path != prefix && !path.starts_with(&prefix_slash) {
                continue;
            }
            let entry = serde_json::from_slice::<PendingFsEntry>(value_guard.value())?;
            to_remove.push((path, entry.doc_id));
        }
        let removed = to_remove.len();
        for (path, doc_id) in to_remove {
            index::remove(&write_txn, doc_id, &path)?;
            table.remove(path.as_str())?;
        }
        removed
    };
    write_txn.commit()?;
    tracing::debug!("Pending FS subtree removed: {} ({})", prefix, removed);
    Ok(removed)
}

/// 清空所有待确认变更
pub fn clear(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        write_txn.delete_table(PENDING_FS_OPS)?;
        let _ = write_txn.open_table(PENDING_FS_OPS)?;
        write_txn.delete_multimap_table(crate::ledger::schema::PENDING_FS_DOC_INDEX)?;
        index::init_table(&write_txn)?;
    }
    write_txn.commit()?;
    tracing::info!("Cleared all pending FS ops");
    Ok(())
}

/// 计算内容的 SipHash 指纹 (64-bit, hex 编码)
///
/// 用于快速判断文件内容是否真正变化（防抖 / 冲突检测）。
/// 注意：非密码学安全哈希，仅用于本地去重。
pub fn content_hash(content: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
