//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 05_diff_logic#authority-diff-core

use crate::ledger::schema::PEER_DOC_SEQ;
use crate::ledger::seq::checked_next_local_seq;
use crate::ledger::{RepoManager, ops};
use crate::models::{DocId, LedgerEntry, Op, PeerId};
use crate::state;
use anyhow::{Result, anyhow};
use redb::ReadableTable;
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

pub(crate) fn append_patch_to_txn(
    write_txn: &redb::WriteTransaction,
    doc_id: DocId,
    peer_label: &str,
    repo_scope: &str,
    patch: &[Op],
) -> Result<()> {
    let peer_id = PeerId::new(peer_label);
    for op in patch {
        let current_local_seq = {
            let peer_seqs = write_txn.open_table(PEER_DOC_SEQ)?;
            peer_seqs
                .get((doc_id.as_u128(), peer_id.as_str()))?
                .map(|value| value.value())
                .unwrap_or(0)
        };
        let next_local_seq = checked_next_local_seq(current_local_seq)
            .ok_or_else(|| anyhow!("LocalSeq overflow"))?;
        let entry = LedgerEntry::new_content(
            doc_id,
            op.clone(),
            chrono::Utc::now().timestamp_millis(),
            peer_id.clone(),
            next_local_seq,
            None,
            None,
        );
        ops::append_op_to_txn(write_txn, &entry, repo_scope)?;
    }
    Ok(())
}
