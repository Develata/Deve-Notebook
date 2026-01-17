use crate::models::{DocId, LedgerEntry};
use crate::state;
use anyhow::Result;
use tracing::info;

/// Compares Ledger state with Disk content.
/// Returns a list of Ops required to make the Ledger match the Disk.
/// Returns None if content is identical.
pub fn compute_reconcile_ops(
    doc_id: DocId,
    ledger_ops: &[LedgerEntry],
    disk_content: &str,
) -> Result<Vec<LedgerEntry>> {
    let ledger_content = state::reconstruct_content(
        &ledger_ops.iter().cloned().map(|e| e.clone()).collect::<Vec<_>>()
    );
    
    // Normalize newlines for comparison
    let disk_norm = disk_content.replace("\r\n", "\n");
    let ledger_norm = ledger_content.replace("\r\n", "\n");

    if disk_norm == ledger_norm {
        return Ok(Vec::new());
    }

    info!("Reconcile: Content mismatch detected for doc {}", doc_id);
    
    let diff_ops = state::compute_diff(&ledger_norm, &disk_norm);
    
    if diff_ops.is_empty() {
        return Ok(Vec::new());
    }

    let now = chrono::Utc::now().timestamp_millis();
    let entries = diff_ops.into_iter().map(|op| LedgerEntry {
        doc_id,
        op,
        timestamp: now,
        peer_id: crate::models::PeerId::new("local_watcher"), // Placeholder for local watcher
        seq: 0, // Placeholder, watcher ops might need real seq management later
    }).collect();

    Ok(entries)
}
