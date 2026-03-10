use crate::ledger::ops;
use crate::models::{DocId, PeerId, StructureOp};
use anyhow::Result;
use redb::Database;

/// Invariants:
/// - synthetic doc id 只允许存在于 Node 结构事件的 ledger append 桥接层。
/// - 结构事件的真实身份仍然由 `StructureOp::node_id()` 决定。
pub fn append_generated_structure_op(
    db: &Database,
    peer_id: PeerId,
    structure: StructureOp,
    timestamp: i64,
) -> Result<(u64, u64)> {
    let doc_id = DocId::from_u128(structure.node_id().as_u128());
    ops::append_generated_op(db, doc_id, peer_id.clone(), move |seq| {
        crate::models::LedgerEntry::new_structure(
            doc_id,
            structure.clone(),
            timestamp,
            peer_id.clone(),
            seq,
        )
    })
}
