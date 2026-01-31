// crates/core/src/ledger/ops.rs
//! # 操作日志模块 (Operations Log)
//!
//! 实现 append-only 操作日志的读写。
//! 支持 Local 和 Shadow 库的隔离写入。

use crate::ledger::schema::*;
use crate::models::{DocId, LedgerEntry};
use anyhow::Result;
use redb::{Database, ReadableMultimapTable, ReadableTable};

/// 追加操作到指定数据库。
pub fn append_op_to_db(db: &Database, entry: &LedgerEntry) -> Result<u64> {
    let write_txn = db.begin_write()?;
    let seq = {
        let mut ops = write_txn.open_table(LEDGER_OPS)?;
        let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;

        let last_seq = ops.last()?.map(|(k, _)| k.value()).unwrap_or(0u64);
        let new_seq = last_seq + 1;
        let bytes = bincode::serialize(entry)?;
        ops.insert(new_seq, bytes.as_slice())?;
        doc_ops.insert(entry.doc_id.as_u128(), new_seq)?;

        // Also update PEER_DOC_SEQ for consistency if it exists
        // (Though append_op_to_db is mostly for shadow re-application where seq is fixed)
        // Check if we need to update max seq for this peer
        let mut peer_seqs = write_txn.open_table(PEER_DOC_SEQ)?;
        let peer_id_str = entry.peer_id.as_str();
        let key = (entry.doc_id.as_u128(), peer_id_str);

        let current_max = peer_seqs.get(key)?.map(|v| v.value()).unwrap_or(0);
        if entry.seq > current_max {
            peer_seqs.insert(key, entry.seq)?;
        }

        new_seq
    };
    write_txn.commit()?;
    Ok(seq)
}

/// 原子生成序号并追加操作
///
/// Returns: (GlobalSeq, LocalSeq)
pub fn append_generated_op(
    db: &Database,
    doc_id: DocId,
    peer_id: crate::models::PeerId,
    mut op_entry_builder: impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    let write_txn = db.begin_write()?;

    // 1. 获取并递增 Local Seq
    let mut peer_seqs = write_txn.open_table(PEER_DOC_SEQ)?;
    let peer_id_str = peer_id.as_str();
    let key = (doc_id.as_u128(), peer_id_str);

    let next_local_seq = if let Some(val) = peer_seqs.get(key)? {
        val.value() + 1
    } else {
        // Lazy Migration: 扫描现有的 ops 找到最大值
        // 注意: 这是一个一次性成本 (One-time cost)
        // 既然我们在这个事务中，我们无法轻易调用外部的 get_ops_from_db 因为被锁住了
        // 我们可以在这里简单扫描 DOC_OPS -> LEDGER_OPS
        // 但为了性能，如果数据量大，这可能会慢。
        // MVP 阶段简单扫描即可。
        let doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
        let ops = write_txn.open_table(LEDGER_OPS)?;
        let seqs = doc_ops.get(doc_id.as_u128())?;

        let mut max_seq = 0;
        for seq in seqs {
            let seq_val = seq?.value();
            if let Some(bytes) = ops.get(seq_val)? {
                // 只反序列化头部? Bincode 不支持部分反序列化。
                // 但我们需要 PeerId
                let entry: LedgerEntry = bincode::deserialize(bytes.value())?;
                if entry.peer_id == peer_id && entry.seq > max_seq {
                    max_seq = entry.seq;
                }
            }
        }
        max_seq + 1
    };

    // 2. 构建 Entry
    let entry = op_entry_builder(next_local_seq);
    if entry.seq != next_local_seq {
        // Sanity Check
        return Err(anyhow::anyhow!("Entry sequence mismatch"));
    }

    // 3. 写入 Ledger
    let mut ops = write_txn.open_table(LEDGER_OPS)?;
    let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;

    let last_global_seq = ops.last()?.map(|(k, _)| k.value()).unwrap_or(0u64);
    let new_global_seq = last_global_seq + 1;
    let bytes = bincode::serialize(&entry)?;

    ops.insert(new_global_seq, bytes.as_slice())?;
    doc_ops.insert(entry.doc_id.as_u128(), new_global_seq)?;

    // 4. 更新 Local Seq Index
    peer_seqs.insert(key, next_local_seq)?;

    // Explicit drops to satisfy borrow checker
    drop(peer_seqs);
    drop(ops);
    drop(doc_ops);

    write_txn.commit()?;
    Ok((new_global_seq, next_local_seq))
}

/// 从指定数据库读取操作。
pub fn get_ops_from_db(db: &Database, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;

    let mut entries = Vec::new();
    let seqs = doc_ops_table.get(doc_id.as_u128())?;

    for seq in seqs {
        let seq_val = seq?.value();
        if let Some(bytes) = ops_table.get(seq_val)? {
            let entry: LedgerEntry = bincode::deserialize(bytes.value())?;
            entries.push((seq_val, entry));
        }
    }

    entries.sort_by_key(|k| k.0);
    Ok(entries)
}

pub fn count_ops_from_db(db: &Database, doc_id: DocId) -> Result<u64> {
    let read_txn = db.begin_read()?;
    let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;
    let mut count = 0u64;
    for item in doc_ops_table.get(doc_id.as_u128())? {
        let _ = item?;
        count += 1;
    }
    Ok(count)
}

pub fn max_seq_from_db(db: &Database, doc_id: DocId) -> Result<u64> {
    let read_txn = db.begin_read()?;
    let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;
    let mut max_seq = 0u64;
    for item in doc_ops_table.get(doc_id.as_u128())? {
        let seq = item?.value();
        if seq > max_seq {
            max_seq = seq;
        }
    }
    Ok(max_seq)
}

/// 从指定数据库读取指定序列号之后的操作。
pub fn get_ops_from_db_after(
    db: &Database,
    doc_id: DocId,
    min_seq: u64,
) -> Result<Vec<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;

    let mut entries = Vec::new();
    for seq in doc_ops_table.get(doc_id.as_u128())? {
        let seq = seq?.value();
        if seq <= min_seq {
            continue;
        }
        if let Some(bytes) = ops_table.get(seq)? {
            let entry: LedgerEntry = bincode::deserialize(bytes.value())?;
            entries.push((seq, entry));
        }
    }

    entries.sort_by_key(|(seq, _)| *seq);
    Ok(entries)
}
