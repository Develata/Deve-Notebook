//! # 操作日志模块 (Operations Log)
//! 
//! 实现 append-only 操作日志的读写。
//! 支持 Local 和 Shadow 库的隔离写入。

use anyhow::Result;
use redb::{Database, ReadableTable, ReadableMultimapTable};
use crate::models::{DocId, LedgerEntry};
use crate::ledger::schema::*;

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
        
        new_seq
    };
    write_txn.commit()?;
    Ok(seq)
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
