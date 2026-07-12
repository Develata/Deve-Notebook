//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage/projection#projection-contract
//!
use crate::ledger::ops;
use crate::models::DocId;
use crate::state;
use anyhow::Result;
use redb::Database;

pub fn verify_snapshot_consistency(
    db: &Database,
    doc_id: DocId,
    seq: u64,
    content: &str,
) -> Result<bool> {
    let entries = ops::get_ops_from_db(db, doc_id)?;
    if entries.is_empty() {
        return Ok(content.is_empty());
    }

    let max_seq = entries.last().map(|(s, _)| *s).unwrap_or(0);
    if seq != max_seq {
        return Ok(false);
    }

    let ops: Vec<_> = entries.into_iter().map(|(_, entry)| entry).collect();
    if state::find_invalid_content_op(&ops).is_some() {
        return Ok(false);
    }

    let rebuilt = state::reconstruct_content(&ops);
    Ok(rebuilt == content)
}
