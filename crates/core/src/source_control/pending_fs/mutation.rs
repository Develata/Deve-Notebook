//! plan_ref:
//!   - 03_storage#watcher-contract
//!   - 05_diff_logic#source-control-runtime

use super::{PendingFsEntry, index};
use crate::ledger::schema::{PENDING_FS_DOC_INDEX, PENDING_FS_OPS};
use crate::models::DocId;
use anyhow::Result;
use redb::{Database, ReadableTable};

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
pub(crate) fn semantic_eq(a: &PendingFsEntry, b: &PendingFsEntry) -> bool {
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
/// 保持原行（含 `detected_at`）字节不变，以满足 plan 04_storage#watcher-contract
/// 的重复信号幂等性要求。
pub fn upsert(db: &Database, entry: &PendingFsEntry) -> Result<()> {
    let written = upsert_many(db, std::slice::from_ref(entry))?;
    if written == 0 {
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

/// 原子插入或更新多条待确认变更。
///
/// 返回实际写入的条目数量；语义完全相同的已有条目会保持字节不变并计为 skipped。
pub fn upsert_many(db: &Database, entries: &[PendingFsEntry]) -> Result<usize> {
    let write_txn = db.begin_write()?;
    let mut written = 0;
    {
        let mut table = write_txn.open_table(PENDING_FS_OPS)?;
        for entry in entries {
            let previous = table
                .get(entry.path.as_str())?
                .map(|guard| serde_json::from_slice::<PendingFsEntry>(guard.value()))
                .transpose()?;
            if previous
                .as_ref()
                .is_some_and(|prev| semantic_eq(prev, entry))
            {
                // Byte-stable idempotency: leave existing row untouched.
                continue;
            }
            let bytes = serde_json::to_vec(entry)?;
            index::replace(
                &write_txn,
                previous.as_ref().and_then(|item| item.doc_id),
                entry.doc_id,
                &entry.path,
            )?;
            table.insert(entry.path.as_str(), bytes.as_slice())?;
            written += 1;
        }
    }
    write_txn.commit()?;
    Ok(written)
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
        write_txn.delete_multimap_table(PENDING_FS_DOC_INDEX)?;
        index::init_table(&write_txn)?;
    }
    write_txn.commit()?;
    tracing::info!("Cleared all pending FS ops");
    Ok(())
}
