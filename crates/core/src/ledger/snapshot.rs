// crates\core\src\ledger
//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage/authority#facts-partition
//!
//! # 快照模块 (Snapshot Management)
//!
//! 管理文档快照的存储与自动清理。

use crate::ledger::schema::*;
use crate::models::DocId;
use anyhow::{Result, anyhow};
use redb::{Database, ReadableMultimapTable};

mod verify;

pub use verify::verify_snapshot_consistency;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotSaveOutcome {
    Saved,
    Inconsistent,
}

/// Save a snapshot for a document (Local DB only).
pub fn save_snapshot(
    db: &Database,
    doc_id: DocId,
    seq: u64,
    content: &str,
    depth: usize,
) -> Result<()> {
    match try_save_snapshot(db, doc_id, seq, content, depth)? {
        SnapshotSaveOutcome::Saved => Ok(()),
        SnapshotSaveOutcome::Inconsistent => Err(anyhow!("Snapshot verification failed")),
    }
}

/// Verify a snapshot candidate once and persist it only when it exactly matches
/// the ledger rebuild. Callers that may safely skip a stale candidate can inspect
/// the typed outcome without repeating the full fold.
pub fn try_save_snapshot(
    db: &Database,
    doc_id: DocId,
    seq: u64,
    content: &str,
    depth: usize,
) -> Result<SnapshotSaveOutcome> {
    if !verify_snapshot_consistency(db, doc_id, seq, content)? {
        return Ok(SnapshotSaveOutcome::Inconsistent);
    }
    let write_txn = db.begin_write()?;
    {
        let mut index = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let mut data = write_txn.open_table(SNAPSHOT_DATA)?;

        data.insert(seq, content.as_bytes())?;
        index.insert(doc_id.as_u128(), seq)?;
    }
    write_txn.commit()?;

    prune_snapshots(db, doc_id, depth)?;
    Ok(SnapshotSaveOutcome::Saved)
}

/// Load the latest snapshot for a document.
///
/// Returns the snapshot sequence number and content if it exists.
pub fn load_latest_snapshot(db: &Database, doc_id: DocId) -> Result<Option<(u64, String)>> {
    let read_txn = db.begin_read()?;

    let index = match read_txn.open_multimap_table(SNAPSHOT_INDEX) {
        Ok(index) => index,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let data = read_txn.open_table(SNAPSHOT_DATA)?;

    let mut latest_seq: Option<u64> = None;
    for item in index.get(doc_id.as_u128())? {
        let seq = item?.value();
        latest_seq = Some(latest_seq.map_or(seq, |cur| cur.max(seq)));
    }

    let seq = match latest_seq {
        Some(seq) => seq,
        None => return Ok(None),
    };

    match data.get(seq)? {
        Some(bytes) => {
            let content = std::str::from_utf8(bytes.value())?.to_owned();
            Ok(Some((seq, content)))
        }
        None => Ok(None),
    }
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
