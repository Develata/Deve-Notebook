// crates/core/src/source_control/snapshot_paths.rs
//! # Snapshot Path Index
//!
//! Maintains the mapping from doc_id to last committed path.

use crate::models::DocId;
use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};

/// 快照路径表定义 (doc_id -> path)
pub const SNAPSHOT_PATHS_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("commit_snapshot_paths");

/// 获取所有快照路径 (doc_id -> path)
pub fn list_snapshot_paths(db: &Database) -> Result<Vec<(DocId, String)>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(SNAPSHOT_PATHS_TABLE)?;
    let mut entries = Vec::new();
    for row in table.iter()? {
        let (doc_id, path) = row?;
        let doc_id = doc_id
            .value()
            .parse::<uuid::Uuid>()
            .map_err(|e| anyhow::anyhow!("Invalid doc_id in snapshot paths: {}", e))?;
        entries.push((DocId(doc_id), path.value().to_string()));
    }
    Ok(entries)
}

/// 根据路径查找快照 doc_id (用于删除文件的 diff)
pub fn find_snapshot_doc_id(db: &Database, path: &str) -> Result<Option<DocId>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(SNAPSHOT_PATHS_TABLE)?;
    for row in table.iter()? {
        let (doc_id, stored_path) = row?;
        if stored_path.value() == path {
            let doc_id = doc_id
                .value()
                .parse::<uuid::Uuid>()
                .map_err(|e| anyhow::anyhow!("Invalid doc_id in snapshot paths: {}", e))?;
            return Ok(Some(DocId(doc_id)));
        }
    }
    Ok(None)
}
