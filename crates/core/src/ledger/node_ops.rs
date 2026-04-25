use crate::ledger::ops;
use crate::ledger::schema::{LEDGER_OPS, NODE_OPS, NODE_PEER_SEQ};
use crate::models::{LedgerEntry, PeerId, StructureOp, deserialize_ledger_entry};
use anyhow::Result;
use redb::{ReadableMultimapTable, ReadableTable, WriteTransaction};

/// Invariants:
/// - 结构事件的真实身份由 `StructureOp` payload 决定。
/// - 目录结构事件不得再伪造 doc id 参与 doc 路由。
pub(crate) fn append_generated_structure_op_to_txn(
    write_txn: &WriteTransaction,
    peer_id: PeerId,
    structure: StructureOp,
    timestamp: i64,
    repo_scope: &str,
) -> Result<(u64, u64)> {
    let node_id = structure.node_id();
    let peer_key = peer_id.as_str().to_string();
    let key = (node_id.as_u128(), peer_key.as_str());
    let next_local_seq = {
        let node_seqs = write_txn.open_table(NODE_PEER_SEQ)?;
        if let Some(val) = node_seqs.get(key)? {
            val.value() + 1
        } else {
            scan_node_peer_seq(write_txn, node_id.as_u128(), &peer_id)? + 1
        }
    };
    let entry = LedgerEntry::new_structure(structure, timestamp, peer_id, next_local_seq);
    let global_seq = ops::append_op_to_txn(write_txn, &entry, repo_scope)?;
    let mut node_seqs = write_txn.open_table(NODE_PEER_SEQ)?;
    node_seqs.insert(key, next_local_seq)?;
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
            let entry: LedgerEntry = deserialize_ledger_entry(bytes.value())?;
            if entry.peer_id == *peer_id && entry.seq > max_seq {
                max_seq = entry.seq;
            }
        }
    }
    Ok(max_seq)
}
