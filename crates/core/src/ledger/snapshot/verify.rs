//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage#projection-contract
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
    sample: bool,
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
    if !sample || rebuilt.len() <= 2048 {
        return Ok(rebuilt == content);
    }

    if rebuilt.chars().count() != content.chars().count() {
        return Ok(false);
    }

    let head = rebuilt.chars().take(1024).collect::<String>();
    let content_head = content.chars().take(1024).collect::<String>();
    if head != content_head {
        return Ok(false);
    }

    let tail: String = rebuilt
        .chars()
        .rev()
        .take(1024)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let content_tail: String = content
        .chars()
        .rev()
        .take(1024)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(tail == content_tail)
}
