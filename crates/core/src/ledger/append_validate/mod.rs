//! plan_ref:
//!   - 04_storage#facts-partition
//!   - 04_storage#projection-contract
//!   - 06_repository#tree-projection-contract
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
    }
}
