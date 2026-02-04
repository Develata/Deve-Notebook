// crates\core\src\ledger
//! # 快照模块 (Snapshot Management)
//!
//! 管理文档快照的存储与自动清理。

use crate::ledger::ops;
use crate::ledger::schema::*;
use crate::models::DocId;
use crate::state;
use anyhow::{Result, anyhow};
use redb::{Database, ReadableMultimapTable};

/// Save a snapshot for a document (Local DB only).
pub fn save_snapshot(
    db: &Database,
    doc_id: DocId,
    seq: u64,
    content: &str,
    depth: usize,
) -> Result<()> {
    let verified = verify_snapshot_consistency(db, doc_id, seq, content, true)?;
    if !verified {
        return Err(anyhow!("Snapshot verification failed"));
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
    Ok(())
}

/// 验证快照内容与账本重建结果一致性。
pub fn verify_snapshot_consistency(
    db: &Database,
    doc_id: DocId,
    seq: u64,
    content: &str,
    sample: bool,
) -> Result<bool> {
    let entries = ops::get_ops_from_db(db, doc_id)?;
    if entries.is_empty() {
        return Ok(content.is_empty());
    }

    let max_seq = entries.last().map(|(s, _)| *s).unwrap_or(0);
    if seq != max_seq {
        return Ok(false);
    }

    let ops: Vec<crate::models::LedgerEntry> =
        entries.iter().map(|(_, entry)| entry.clone()).collect();
    let rebuilt = state::reconstruct_content(&ops);
    if !sample || rebuilt.len() <= 2048 {
        return Ok(rebuilt == content);
    }

    if rebuilt.chars().count() != content.chars().count() {
        return Ok(false);
    }

    let head = rebuilt.chars().take(1024).collect::<String>();
    let tail = rebuilt.chars().rev().take(1024).collect::<Vec<_>>();
    let tail: String = tail.into_iter().rev().collect();

    let content_head = content.chars().take(1024).collect::<String>();
    let content_tail = content.chars().rev().take(1024).collect::<Vec<_>>();
    let content_tail: String = content_tail.into_iter().rev().collect();

    Ok(head == content_head && tail == content_tail)
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
