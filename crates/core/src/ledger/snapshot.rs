//! # 快照模块 (Snapshot Management)
//! 
//! 管理文档快照的存储与自动清理。

use anyhow::Result;
use redb::{Database, ReadableTable, ReadableMultimapTable};
use crate::models::DocId;
use crate::ledger::schema::*;

/// Save a snapshot for a document (Local DB only).
pub fn save_snapshot(
    db: &Database, 
    doc_id: DocId, 
    seq: u64, 
    content: &str,
    depth: usize
) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut index = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let mut data = write_txn.open_table(SNAPSHOT_DATA)?;
        
        data.insert(seq, content.as_bytes())?;
        index.insert(doc_id.as_u128(), seq)?;
    }
    write_txn.commit()?;
    
    prune_snapshots(db, doc_id, depth)?;
    Ok(())
}

/// Prune old snapshots if they exceed the configured depth.
fn prune_snapshots(db: &Database, doc_id: DocId, depth: usize) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut snapshots = Vec::new();
        {
            let index = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
            let iter = index.get(doc_id.as_u128())?;
            for item in iter {
                let seq: u64 = item?.value();
                snapshots.push(seq);
            }
        }
        
        snapshots.sort();
        let total = snapshots.len();
        if total > depth {
            let to_remove = total - depth;
            let remove_seqs = &snapshots[0..to_remove];
            
            let mut index = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
            let mut data = write_txn.open_table(SNAPSHOT_DATA)?;
            
            for &seq in remove_seqs {
                index.remove(doc_id.as_u128(), seq)?;
                data.remove(seq)?;
            }
        }
    }
    write_txn.commit()?;
    Ok(())
}
