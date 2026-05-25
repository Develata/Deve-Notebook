//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/projection#projection-contract
//!
use crate::models::{DocId, LedgerEntry, deserialize_ledger_entry};
use crate::state::ContentOpValidator;
use anyhow::{Result, anyhow};
use redb::{ReadableMultimapTable, ReadableTable, WriteTransaction};

use super::errors::{reject_invalid_content, reject_missing_doc_id};
use crate::ledger::schema::{DOC_OPS, LEDGER_OPS};

pub(super) fn validate_content_append(
    write_txn: &WriteTransaction,
    entry: &LedgerEntry,
    repo_scope: &str,
) -> Result<()> {
    let Some(doc_id) = entry.doc_id else {
        return Err(reject_missing_doc_id(entry, repo_scope));
    };
    let mut validator = ContentOpValidator::default();
    validate_existing_doc_entries(write_txn, doc_id, entry, repo_scope, &mut validator)?;
    if let Some(issue) = validator.push_entry(entry) {
        return Err(reject_invalid_content(
            doc_id, entry, &issue, false, repo_scope,
        ));
    }
    Ok(())
}

fn validate_existing_doc_entries(
    write_txn: &WriteTransaction,
    doc_id: DocId,
    new_entry: &LedgerEntry,
    repo_scope: &str,
    validator: &mut ContentOpValidator,
) -> Result<()> {
    let doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
    let ops = write_txn.open_table(LEDGER_OPS)?;
    let mut seqs = Vec::new();
    for seq in doc_ops.get(doc_id.as_u128())? {
        seqs.push(seq?.value());
    }
    seqs.sort_unstable();

    for global_seq in seqs {
        let Some(bytes) = ops.get(global_seq)? else {
            return Err(anyhow!(
                "Broken DOC_OPS index for {}: missing ledger op at seq {}",
                doc_id,
                global_seq
            ));
        };
        let entry = deserialize_ledger_entry(bytes.value())?;
        if let Some(issue) = validator.push_entry(&entry) {
            return Err(reject_invalid_content(
                doc_id, new_entry, &issue, true, repo_scope,
            ));
        }
    }
    Ok(())
}
