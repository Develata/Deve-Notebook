use crate::ledger::schema::{DOC_OPS, LEDGER_OPS};
use crate::models::{DocId, LedgerEntry, deserialize_ledger_entry};
use crate::state::{InvalidContentOp, describe_invalid_content_op, find_invalid_content_op};
use anyhow::{Result, anyhow};
use redb::{ReadableMultimapTable, ReadableTable, WriteTransaction};

pub fn validate_content_append(
    write_txn: &WriteTransaction,
    entry: &LedgerEntry,
    repo_scope: &str,
) -> Result<()> {
    let Some(doc_id) = entry.doc_id else {
        return match entry.content_op() {
            Some(_) => Err(reject_missing_doc_id(entry, repo_scope)),
            None => Ok(()),
        };
    };
    if entry.content_op().is_none() {
        return Ok(());
    }
    let mut ops = load_doc_entries(write_txn, doc_id)?;
    if let Some(issue) = find_invalid_content_op(&ops) {
        return Err(reject_invalid_append(
            doc_id, entry, &issue, true, repo_scope,
        ));
    }
    ops.push(entry.clone());
    if let Some(issue) = find_invalid_content_op(&ops) {
        return Err(reject_invalid_append(
            doc_id, entry, &issue, false, repo_scope,
        ));
    }
    Ok(())
}

fn load_doc_entries(write_txn: &WriteTransaction, doc_id: DocId) -> Result<Vec<LedgerEntry>> {
    let doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
    let ops = write_txn.open_table(LEDGER_OPS)?;
    let mut entries = Vec::new();
    for seq in doc_ops.get(doc_id.as_u128())? {
        let global_seq = seq?.value();
        let Some(bytes) = ops.get(global_seq)? else {
            return Err(broken_doc_ops_index(doc_id, global_seq));
        };
        entries.push((global_seq, deserialize_ledger_entry(bytes.value())?));
    }
    entries.sort_by_key(|(global_seq, _)| *global_seq);
    Ok(entries.into_iter().map(|(_, entry)| entry).collect())
}

fn broken_doc_ops_index(doc_id: DocId, seq: u64) -> anyhow::Error {
    anyhow!(
        "Broken DOC_OPS index for {}: missing ledger op at seq {}",
        doc_id,
        seq
    )
}

fn reject_missing_doc_id(entry: &LedgerEntry, repo_scope: &str) -> anyhow::Error {
    let issue = "content op missing doc id";
    tracing::warn!(
        repo_scope,
        doc_id = "<missing>",
        peer_id = %entry.peer_id,
        seq = entry.seq,
        issue,
        "Rejecting invalid content append"
    );
    anyhow!("Content op missing doc id")
}

fn reject_invalid_append(
    doc_id: DocId,
    entry: &LedgerEntry,
    issue: &InvalidContentOp,
    existing_history_invalid: bool,
    repo_scope: &str,
) -> anyhow::Error {
    let issue_text = describe_invalid_content_op(issue);
    tracing::warn!(
        repo_scope,
        doc_id = %doc_id,
        peer_id = %entry.peer_id,
        seq = entry.seq,
        issue = %issue_text,
        existing_history_invalid,
        "Rejecting invalid content append"
    );
    if existing_history_invalid {
        anyhow!(
            "Refusing to append content op for {}: existing history invalid: {}",
            doc_id,
            issue_text
        )
    } else {
        anyhow!(
            "Refusing to append content op for {}: {}",
            doc_id,
            issue_text
        )
    }
}
