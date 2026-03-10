use crate::ledger::schema::*;
use crate::models::{DocId, LedgerEntry, NodeId};
use anyhow::Result;
use redb::{Database, ReadableMultimapTable, ReadableTable};

pub fn append_op_to_db(db: &Database, entry: &LedgerEntry) -> Result<u64> {
    let write_txn = db.begin_write()?;
    let seq = {
        let mut ops = write_txn.open_table(LEDGER_OPS)?;
        let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
        let mut node_ops = write_txn.open_multimap_table(NODE_OPS)?;

        let last_seq = ops.last()?.map(|(k, _)| k.value()).unwrap_or(0u64);
        let new_seq = last_seq + 1;
        let bytes = bincode::serialize(entry)?;
        ops.insert(new_seq, bytes.as_slice())?;
        doc_ops.insert(entry.doc_id.as_u128(), new_seq)?;
        if let Some(node_id) = entry.structure_node_id() {
            node_ops.insert(node_id.as_u128(), new_seq)?;
        }

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

pub fn append_generated_op(
    db: &Database,
    doc_id: DocId,
    peer_id: crate::models::PeerId,
    mut op_entry_builder: impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    append_generated_op_inner(db, doc_id, peer_id, None, &mut op_entry_builder)
}

pub fn append_generated_client_op(
    db: &Database,
    doc_id: DocId,
    peer_id: crate::models::PeerId,
    client_id: u64,
    client_op_id: u64,
    mut op_entry_builder: impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    append_generated_op_inner(
        db,
        doc_id,
        peer_id,
        Some((client_id, client_op_id)),
        &mut op_entry_builder,
    )
}

fn append_generated_op_inner(
    db: &Database,
    doc_id: DocId,
    peer_id: crate::models::PeerId,
    client_ref: Option<(u64, u64)>,
    op_entry_builder: &mut impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    let write_txn = db.begin_write()?;

    let mut peer_seqs = write_txn.open_table(PEER_DOC_SEQ)?;
    let peer_id_str = peer_id.as_str();
    let key = (doc_id.as_u128(), peer_id_str);

    let next_local_seq = if let Some(val) = peer_seqs.get(key)? {
        val.value() + 1
    } else {
        let doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
        let ops = write_txn.open_table(LEDGER_OPS)?;
        let seqs = doc_ops.get(doc_id.as_u128())?;

        let mut max_seq = 0;
        for seq in seqs {
            let seq_val = seq?.value();
            if let Some(bytes) = ops.get(seq_val)? {
                let entry: LedgerEntry = bincode::deserialize(bytes.value())?;
                if entry.peer_id == peer_id && entry.seq > max_seq {
                    max_seq = entry.seq;
                }
            }
        }
        max_seq + 1
    };

    let entry = op_entry_builder(next_local_seq);
    if entry.seq != next_local_seq {
        return Err(anyhow::anyhow!("Entry sequence mismatch"));
    }

    let mut ops = write_txn.open_table(LEDGER_OPS)?;
    let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
    let mut node_ops = write_txn.open_multimap_table(NODE_OPS)?;
    let mut client_ops = write_txn.open_table(CLIENT_OP_INDEX)?;

    let last_global_seq = ops.last()?.map(|(k, _)| k.value()).unwrap_or(0u64);
    let new_global_seq = last_global_seq + 1;
    let bytes = bincode::serialize(&entry)?;

    ops.insert(new_global_seq, bytes.as_slice())?;
    doc_ops.insert(entry.doc_id.as_u128(), new_global_seq)?;
    if let Some(node_id) = entry.structure_node_id() {
        node_ops.insert(node_id.as_u128(), new_global_seq)?;
    }
    if let Some((client_id, client_op_id)) = client_ref {
        client_ops.insert(
            (entry.doc_id.as_u128(), client_id, client_op_id),
            new_global_seq,
        )?;
    }

    peer_seqs.insert(key, next_local_seq)?;

    drop(peer_seqs);
    drop(ops);
    drop(doc_ops);
    drop(node_ops);
    drop(client_ops);

    write_txn.commit()?;
    Ok((new_global_seq, next_local_seq))
}

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
            entries.push((seq, bincode::deserialize(bytes.value())?));
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
            let entry: LedgerEntry = bincode::deserialize(bytes.value())?;
            entries.push((seq, entry));
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
    let entry: LedgerEntry = bincode::deserialize(bytes.value())?;
    Ok(Some((global_seq, entry)))
}
