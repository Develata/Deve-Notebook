use crate::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, PEER_DOC_SEQ};
use crate::models::LedgerEntry;
use anyhow::Result;
use redb::{Database, ReadableTable};

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
        if let Some(doc_id) = entry.doc_id {
            doc_ops.insert(doc_id.as_u128(), new_seq)?;
        }
        if let Some(node_id) = entry.structure_node_id() {
            node_ops.insert(node_id.as_u128(), new_seq)?;
        }
        let mut peer_seqs = write_txn.open_table(PEER_DOC_SEQ)?;
        if let Some(doc_id) = entry.doc_id {
            let key = (doc_id.as_u128(), entry.peer_id.as_str());
            let current_max = peer_seqs.get(key)?.map(|v| v.value()).unwrap_or(0);
            if entry.seq > current_max {
                peer_seqs.insert(key, entry.seq)?;
            }
        }
        new_seq
    };
    write_txn.commit()?;
    Ok(seq)
}
