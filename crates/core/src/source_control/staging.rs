// crates\core\src\source_control
//! # 暂存区管理 (Staging Manager)
//!
//! 管理文件的暂存状态，持久化到数据库。
//!
//! **存储结构**:
//! - Table: `staged_files` - 存储已暂存的文件路径及其变更元数据

#[path = "staging_index.rs"]
mod index;
#[path = "staging_query.rs"]
mod query;

use crate::models::DocId;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::PendingFsEntry;
use anyhow::Result;
pub use query::{
    get_staged, is_staged, list_staged, list_staged_entries, list_staged_entries_for_doc,
    list_staged_with_status,
};
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

/// 暂存区表定义 (path -> JSON bytes)
pub const STAGED_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("staged_files");

/// 暂存条目（包含变更状态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedEntry {
    pub timestamp: i64,
    #[serde(default)]
    pub renamed_from: Option<String>,
    #[serde(default)]
    pub doc_id: Option<DocId>,
    pub status: ChangeStatus,
    pub content_hash: String,
    pub has_conflict: bool,
}

/// 初始化暂存区表
pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(STAGED_TABLE)?;
        index::init_table(&write_txn)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 将一条 pending 记录整体移入暂存区。
pub fn stage_pending_entry(db: &Database, entry: &PendingFsEntry) -> Result<()> {
    let staged = StagedEntry {
        timestamp: chrono::Utc::now().timestamp_millis(),
        renamed_from: entry.renamed_from.clone(),
        doc_id: entry.doc_id,
        status: entry.change_type,
        content_hash: entry.content_hash.clone(),
        has_conflict: entry.has_conflict,
    };
    stage_entry(db, &entry.path, &staged)
}

/// 移除并返回单条暂存记录
pub fn take_staged(db: &Database, path: &str) -> Result<Option<StagedEntry>> {
    let existing = get_staged(db, path)?;
    if existing.is_none() {
        return Ok(None);
    }
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(STAGED_TABLE)?;
        index::remove(
            &write_txn,
            existing.as_ref().and_then(|entry| entry.doc_id),
            path,
        )?;
        table.remove(path)?;
    }
    write_txn.commit()?;
    Ok(existing)
}

fn stage_entry(db: &Database, path: &str, entry: &StagedEntry) -> Result<()> {
    let bytes = serde_json::to_vec(&entry)?;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(STAGED_TABLE)?;
        let previous = table
            .get(path)?
            .map(|guard| serde_json::from_slice::<StagedEntry>(guard.value()))
            .transpose()?;
        index::replace(
            &write_txn,
            previous.as_ref().and_then(|item| item.doc_id),
            entry.doc_id,
            path,
        )?;
        table.insert(path, bytes.as_slice())?;
    }
    write_txn.commit()?;
    tracing::info!("Staged file: {} ({:?})", path, entry.status);
    Ok(())
}

/// 清空暂存区 (提交后调用)
pub fn clear(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        write_txn.delete_table(STAGED_TABLE)?;
        let _ = write_txn.open_table(STAGED_TABLE)?;
        write_txn.delete_multimap_table(crate::ledger::schema::STAGED_DOC_INDEX)?;
        index::init_table(&write_txn)?;
    }
    write_txn.commit()?;
    tracing::info!("Cleared staging area");
    Ok(())
}
