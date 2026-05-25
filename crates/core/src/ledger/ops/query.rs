//! plan_ref:
//!   - 03_storage#facts-partition
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract

use crate::ledger::schema::{CLIENT_OP_INDEX, DOC_OPS, LEDGER_OPS, NODE_OPS};
use crate::models::{DocId, LedgerEntry, NodeId, deserialize_ledger_entry};
use anyhow::{Result, anyhow};
use redb::Database;

/// Invariants:
/// - `DOC_OPS` 中出现的每个序号都必须在 `LEDGER_OPS` 中存在。
///
/// Post-conditions:
/// - 一旦发现悬空 side index，立即 fail-closed，而不是静默跳过。
pub fn get_ops_from_db(db: &Database, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;
    let mut entries = Vec::new();
    for seq in doc_ops_table.get(doc_id.as_u128())? {
        let seq_val = seq?.value();
        let Some(bytes) = ops_table.get(seq_val)? else {
            return Err(broken_doc_ops_index(doc_id, seq_val));
        };
        entries.push((seq_val, deserialize_ledger_entry(bytes.value())?));
    }
    entries.sort_by_key(|k| k.0);
    Ok(entries)
}

/// Invariants:
/// - `NODE_OPS` 中出现的每个序号都必须在 `LEDGER_OPS` 中存在。
///
/// Post-conditions:
/// - 一旦发现悬空结构索引，立即 fail-closed。
pub fn get_structure_ops_for_node_from_db(
    db: &Database,
    node_id: NodeId,
) -> Result<Vec<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let node_ops_table = read_txn.open_multimap_table(NODE_OPS)?;
    let mut entries = Vec::new();
    for seq in node_ops_table.get(node_id.as_u128())? {
        let seq = seq?.value();
        let Some(bytes) = ops_table.get(seq)? else {
            return Err(broken_node_ops_index(node_id, seq));
        };
        entries.push((seq, deserialize_ledger_entry(bytes.value())?));
    }
    entries.sort_by_key(|(seq, _)| *seq);
    Ok(entries)
}

pub fn count_ops_from_db(db: &Database, doc_id: DocId) -> Result<u64> {
    let read_txn = db.begin_read()?;
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;
    let mut count = 0u64;
    for item in doc_ops_table.get(doc_id.as_u128())? {
        let seq = item?.value();
        if ops_table.get(seq)?.is_none() {
            return Err(broken_doc_ops_index(doc_id, seq));
        }
        count += 1;
    }
    Ok(count)
}

pub fn max_seq_from_db(db: &Database, doc_id: DocId) -> Result<u64> {
    let read_txn = db.begin_read()?;
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;
    let mut max_seq = 0u64;
    for item in doc_ops_table.get(doc_id.as_u128())? {
        let seq = item?.value();
        if ops_table.get(seq)?.is_none() {
            return Err(broken_doc_ops_index(doc_id, seq));
        }
        if seq > max_seq {
            max_seq = seq;
        }
    }
    Ok(max_seq)
}

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
        let Some(bytes) = ops_table.get(seq)? else {
            return Err(broken_doc_ops_index(doc_id, seq));
        };
        entries.push((seq, deserialize_ledger_entry(bytes.value())?));
    }
    entries.sort_by_key(|(seq, _)| *seq);
    Ok(entries)
}

pub fn find_client_op_in_db(
    db: &Database,
    client_id: u64,
    client_op_id: u64,
) -> Result<Option<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let client_ops = match read_txn.open_table(CLIENT_OP_INDEX) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(broken_client_op_index("table missing"));
        }
        Err(err) => return Err(err.into()),
    };
    let Some(global_seq) = client_ops
        .get((client_id, client_op_id))?
        .map(|seq| seq.value())
    else {
        return Ok(None);
    };
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let Some(bytes) = ops_table.get(global_seq)? else {
        return Err(broken_client_op_index(format!(
            "missing ledger op at seq {}",
            global_seq
        )));
    };
    let entry = deserialize_ledger_entry(bytes.value())?;
    if entry.doc_id.is_none()
        || entry.client_id != Some(client_id)
        || entry.client_op_id != Some(client_op_id)
    {
        return Err(broken_client_op_index(format!(
            "metadata mismatch at seq {} for client op ({}, {})",
            global_seq, client_id, client_op_id
        )));
    }
    Ok(Some((global_seq, entry)))
}

fn broken_client_op_index(detail: impl Into<String>) -> anyhow::Error {
    anyhow!("Broken client op index: {}", detail.into())
}

fn broken_doc_ops_index(doc_id: DocId, seq: u64) -> anyhow::Error {
    anyhow!(
        "Broken DOC_OPS index for {}: missing ledger op at seq {}",
        doc_id,
        seq
    )
}

fn broken_node_ops_index(node_id: NodeId, seq: u64) -> anyhow::Error {
    anyhow!(
        "Broken NODE_OPS index for {}: missing ledger op at seq {}",
        node_id,
        seq
    )
}
