use super::validate;
use crate::ledger::schema::{CLIENT_OP_INDEX, DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_DOC_SEQ};
use crate::models::{DocId, LedgerEntry, PeerId, deserialize_ledger_entry};
use anyhow::Result;
use redb::{Database, ReadableMultimapTable, ReadableTable};

/// Returns: `(GlobalSeq, LocalSeq)`
pub fn append_generated_op(
    db: &Database,
    doc_id: DocId,
    peer_id: PeerId,
    repo_scope: &str,
    mut op_entry_builder: impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    append_generated_op_inner(db, doc_id, peer_id, repo_scope, None, &mut op_entry_builder)
}

pub fn append_generated_client_op(
    db: &Database,
    doc_id: DocId,
    peer_id: PeerId,
    repo_scope: &str,
    client_id: u64,
    client_op_id: u64,
    mut op_entry_builder: impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    append_generated_op_inner(
        db,
        doc_id,
        peer_id,
        repo_scope,
        Some((client_id, client_op_id)),
        &mut op_entry_builder,
    )
}

fn append_generated_op_inner(
    db: &Database,
    doc_id: DocId,
    peer_id: PeerId,
    repo_scope: &str,
    client_ref: Option<(u64, u64)>,
    op_entry_builder: &mut impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    let write_txn = db.begin_write()?;
    let mut peer_seqs = write_txn.open_table(PEER_DOC_SEQ)?;
    let key = (doc_id.as_u128(), peer_id.as_str());
    let next_local_seq = if let Some(val) = peer_seqs.get(key)? {
        val.value() + 1
    } else {
        scan_existing_peer_seq(&write_txn, doc_id, &peer_id)? + 1
    };

    let entry = op_entry_builder(next_local_seq);
    if entry.seq != next_local_seq {
        return Err(anyhow::anyhow!("Entry sequence mismatch"));
    }
    validate::validate_ledger_append(&write_txn, &entry, repo_scope)?;

    let mut ops = write_txn.open_table(LEDGER_OPS)?;
    let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
    let mut node_ops = write_txn.open_multimap_table(NODE_OPS)?;
    let mut client_ops = write_txn.open_table(CLIENT_OP_INDEX)?;
    let last_global_seq = ops.last()?.map(|(k, _)| k.value()).unwrap_or(0u64);
    let new_global_seq = last_global_seq + 1;
    let bytes = bincode::serialize(&entry)?;

    ops.insert(new_global_seq, bytes.as_slice())?;
    if let Some(doc_id) = entry.doc_id {
        doc_ops.insert(doc_id.as_u128(), new_global_seq)?;
    }
    if let Some(node_id) = entry.structure_node_id() {
        node_ops.insert(node_id.as_u128(), new_global_seq)?;
    }
    if let Some((client_id, client_op_id)) = client_ref {
        let doc_id = entry
            .doc_id
            .ok_or_else(|| anyhow::anyhow!("client op missing doc id"))?;
        client_ops.insert((doc_id.as_u128(), client_id, client_op_id), new_global_seq)?;
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

fn scan_existing_peer_seq(
    write_txn: &redb::WriteTransaction,
    doc_id: DocId,
    peer_id: &PeerId,
) -> Result<u64> {
    let doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
    let ops = write_txn.open_table(LEDGER_OPS)?;
    let mut max_seq = 0;
    for seq in doc_ops.get(doc_id.as_u128())? {
        let seq_val = seq?.value();
        if let Some(bytes) = ops.get(seq_val)? {
            let entry: LedgerEntry = deserialize_ledger_entry(bytes.value())?;
            if entry.peer_id == *peer_id && entry.seq > max_seq {
                max_seq = entry.seq;
            }
        }
    }
    Ok(max_seq)
}
