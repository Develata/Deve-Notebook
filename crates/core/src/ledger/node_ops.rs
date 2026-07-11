//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 04_repository#tree-projection-contract
//!
use crate::ledger::ops;
use crate::models::{FactActor, LedgerEntry, PeerId, StructureOp};
use anyhow::Result;
use redb::WriteTransaction;

/// Invariants:
/// - 结构事件的真实身份由 `StructureOp` payload 决定。
/// - 目录结构事件不得再伪造 doc id 参与 doc 路由。
pub(crate) fn append_generated_structure_op_to_txn(
    write_txn: &WriteTransaction,
    peer_id: PeerId,
    actor: FactActor,
    structure: StructureOp,
    timestamp: i64,
    repo_scope: &str,
) -> Result<(u64, u64)> {
    let next_peer_seq = ops::write_direct::next_peer_fact_seq(write_txn, &peer_id)?;
    let entry =
        LedgerEntry::new_structure_with_actor(structure, timestamp, peer_id, next_peer_seq, actor);
    let global_seq = ops::append_op_to_txn(write_txn, &entry, repo_scope)?;
    Ok((global_seq, next_peer_seq.get()))
}
