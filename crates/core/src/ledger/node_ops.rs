use crate::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS, NODE_PEER_SEQ};
use crate::models::{DocId, LedgerEntry, PeerId, StructureOp};
use anyhow::Result;
use redb::{Database, ReadableMultimapTable, ReadableTable};

/// Invariants:
/// - synthetic doc id 只允许存在于 Node 结构事件的 ledger append 桥接层。
/// - 结构事件的真实身份仍然由 `StructureOp::node_id()` 决定。
pub fn append_generated_structure_op(
    db: &Database,
    peer_id: PeerId,
    structure: StructureOp,
    timestamp: i64,
) -> Result<(u64, u64)> {
    let node_id = structure.node_id();
    let doc_id = DocId::from_u128(node_id.as_u128());
    let peer_key = peer_id.as_str().to_string();
    let write_txn = db.begin_write()?;
    let mut node_seqs = write_txn.open_table(NODE_PEER_SEQ)?;
    let key = (node_id.as_u128(), peer_key.as_str());
    let next_local_seq = if let Some(val) = node_seqs.get(key)? {
        val.value() + 1
    } else {
        scan_node_peer_seq(&write_txn, node_id.as_u128(), &peer_id)? + 1
    };
    let entry = LedgerEntry::new_structure(doc_id, structure, timestamp, peer_id, next_local_seq);
    let mut ops = write_txn.open_table(LEDGER_OPS)?;
    let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
    let mut node_ops = write_txn.open_multimap_table(NODE_OPS)?;
    let global_seq = ops.last()?.map(|(k, _)| k.value() + 1).unwrap_or(1);
    let bytes = bincode::serialize(&entry)?;
    ops.insert(global_seq, bytes.as_slice())?;
    doc_ops.insert(doc_id.as_u128(), global_seq)?;
    node_ops.insert(node_id.as_u128(), global_seq)?;
    node_seqs.insert(key, next_local_seq)?;
    drop(node_seqs);
    drop(ops);
    drop(doc_ops);
    drop(node_ops);
    write_txn.commit()?;
    Ok((global_seq, next_local_seq))
}

fn scan_node_peer_seq(
    write_txn: &redb::WriteTransaction,
    node_id: u128,
    peer_id: &PeerId,
) -> Result<u64> {
    let node_ops = write_txn.open_multimap_table(NODE_OPS)?;
    let ops = write_txn.open_table(LEDGER_OPS)?;
    let mut max_seq = 0;
    for seq in node_ops.get(node_id)? {
        let seq = seq?.value();
        if let Some(bytes) = ops.get(seq)? {
            let entry: LedgerEntry = bincode::deserialize(bytes.value())?;
            if entry.peer_id == *peer_id && entry.seq > max_seq {
                max_seq = entry.seq;
            }
        }
    }
    Ok(max_seq)
}
