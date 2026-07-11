//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/projection#projection-contract
//!   - 04_repository#tree-projection-contract
//!
mod content;
mod errors;
mod projection;
mod structure;

use crate::models::{LedgerEntry, LedgerEvent};
use anyhow::Result;
use redb::WriteTransaction;

pub(crate) fn validate_ledger_append(
    write_txn: &WriteTransaction,
    entry: &LedgerEntry,
    repo_scope: &str,
) -> Result<()> {
    match &entry.event {
        LedgerEvent::Content(_) => content::validate_content_append(write_txn, entry, repo_scope),
        LedgerEvent::Structure(op) => {
            structure::validate_structure_append(write_txn, op, repo_scope)
        }
        LedgerEvent::MergeAnchor(anchor) => {
            if entry.doc_id.is_none() {
                anyhow::bail!("MergeAnchor missing doc_id");
            }
            if anchor.source_peer_id.as_str().is_empty() {
                anyhow::bail!("MergeAnchor source peer must not be empty");
            }
            if anchor.source_peer_id == entry.origin_peer_id {
                anyhow::bail!("MergeAnchor source peer must differ from local origin");
            }
            if anchor.source_waterline == 0 {
                anyhow::bail!("MergeAnchor source waterline must be positive");
            }
            if entry.client_id.is_some() || entry.client_op_id.is_some() {
                anyhow::bail!("MergeAnchor must not carry browser client identity");
            }
            Ok(())
        }
    }
}
