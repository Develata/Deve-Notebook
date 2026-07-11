//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 05_diff_logic#authority-diff-core

use crate::ledger::{RepoManager, ops};
use crate::models::{DocId, FactActor, LedgerEntry, Op, PeerId};
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
    let actor = FactActor::new(peer_label)?;
    let writer = repo.local_fact_writer(actor);
    for op in ops {
        writer.append_content_in_local_repo(
            repo_name,
            doc_id,
            op.clone(),
            chrono::Utc::now().timestamp_millis(),
        )?;
    }
    info!("Reconcile: Applied {} ops for doc {}", ops.len(), doc_id);
    Ok(())
}

pub(crate) fn append_patch_to_txn(
    write_txn: &redb::WriteTransaction,
    doc_id: DocId,
    origin_peer_id: &PeerId,
    peer_label: &str,
    repo_scope: &str,
    patch: &[Op],
) -> Result<()> {
    let actor = FactActor::new(peer_label)?;
    for op in patch {
        let next_peer_seq = ops::write_direct::next_peer_fact_seq(write_txn, origin_peer_id)?;
        let entry = LedgerEntry::new_content_with_actor(
            doc_id,
            op.clone(),
            chrono::Utc::now().timestamp_millis(),
            origin_peer_id.clone(),
            next_peer_seq,
            actor.clone(),
            None,
            None,
        );
        ops::append_op_to_txn(write_txn, &entry, repo_scope)?;
    }
    Ok(())
}
