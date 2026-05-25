//! plan_ref:
//!   - 03_storage#projection-contract
//!   - 05_diff_logic#authority-diff-core

use crate::ledger::RepoManager;
use crate::models::{DocId, LedgerEntry, Op, PeerId};
use crate::state;
use anyhow::{Result, anyhow};
use tracing::info;

/// Compares Ledger state with target content.
/// Returns a diff patch that can be appended with generated local seq.
pub fn compute_reconcile_patch(
    ledger_ops: &[LedgerEntry],
    target_content: &str,
) -> Result<Vec<Op>> {
    if let Some(issue) = state::find_invalid_content_op(ledger_ops) {
        return Err(anyhow!(
            "invalid ledger content ops while reconciling: {}",
            state::describe_invalid_content_op(&issue)
        ));
    }

    let ledger_content = state::reconstruct_content(ledger_ops);

    let target_norm = target_content.replace("\r\n", "\n");
    let ledger_norm = ledger_content.replace("\r\n", "\n");

    if target_norm == ledger_norm {
        return Ok(Vec::new());
    }

    Ok(state::compute_diff(&ledger_norm, &target_norm))
}

pub fn append_patch_in_local_repo(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
    peer_label: &str,
    ops: &[Op],
) -> Result<()> {
    let peer_id = PeerId::new(peer_label);
    for op in ops {
        let op = op.clone();
        let peer_id = peer_id.clone();
        let timestamp = chrono::Utc::now().timestamp_millis();
        repo.append_generated_op_in_local_repo(repo_name, doc_id, peer_id.clone(), move |seq| {
            LedgerEntry::new_content(
                doc_id,
                op.clone(),
                timestamp,
                peer_id.clone(),
                seq,
                None,
                None,
            )
        })?;
    }
    info!("Reconcile: Applied {} ops for doc {}", ops.len(), doc_id);
    Ok(())
}
