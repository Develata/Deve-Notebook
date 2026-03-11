use crate::ledger::schema::{CLIENT_OP_INDEX, DOC_OPS, LEDGER_OPS, NODE_OPS};
use crate::models::{DocId, LedgerEntry, NodeId, deserialize_ledger_entry};
use anyhow::Result;
use redb::Database;

pub fn get_ops_from_db(db: &Database, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;
    let mut entries = Vec::new();
    for seq in doc_ops_table.get(doc_id.as_u128())? {
        let seq_val = seq?.value();
        if let Some(bytes) = ops_table.get(seq_val)? {
            entries.push((seq_val, deserialize_ledger_entry(bytes.value())?));
        }
    }
    entries.sort_by_key(|k| k.0);
    Ok(entries)
}

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
        if let Some(bytes) = ops_table.get(seq)? {
            entries.push((seq, deserialize_ledger_entry(bytes.value())?));
        }
    }
    entries.sort_by_key(|(seq, _)| *seq);
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
            entries.push((seq, deserialize_ledger_entry(bytes.value())?));
        }
    }
    entries.sort_by_key(|(seq, _)| *seq);
    Ok(entries)
}

pub fn find_client_op_in_db(
    db: &Database,
    doc_id: DocId,
    client_id: u64,
    client_op_id: u64,
) -> Result<Option<(u64, LedgerEntry)>> {
    let read_txn = db.begin_read()?;
    let client_ops = match read_txn.open_table(CLIENT_OP_INDEX) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let Some(global_seq) = client_ops
        .get((doc_id.as_u128(), client_id, client_op_id))?
        .map(|seq| seq.value())
    else {
        return Ok(None);
    };
    let ops_table = read_txn.open_table(LEDGER_OPS)?;
    let Some(bytes) = ops_table.get(global_seq)? else {
        return Ok(None);
    };
    Ok(Some((global_seq, deserialize_ledger_entry(bytes.value())?)))
}
