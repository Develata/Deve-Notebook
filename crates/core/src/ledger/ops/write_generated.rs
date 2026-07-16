//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!
use crate::ledger::runtime_tables;
use crate::ledger::schema::CLIENT_OP_INDEX;
use crate::models::{DocId, LedgerEntry, LedgerEvent, PeerId};
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable};

/// Returns: `(GlobalSeq storage key, LocalSeq)`.
pub fn append_generated_op(
    db: &Database,
    doc_id: DocId,
    peer_id: PeerId,
    repo_scope: &str,
    mut op_entry_builder: impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    append_generated_op_inner(db, doc_id, peer_id, repo_scope, None, &mut op_entry_builder)
}

pub fn append_generated_client_op(
    db: &Database,
    doc_id: DocId,
    peer_id: PeerId,
    repo_scope: &str,
    client_id: u64,
    client_op_id: u64,
    mut op_entry_builder: impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    runtime_tables::repair_client_op_index_if_missing(db)?;
    append_generated_op_inner(
        db,
        doc_id,
        peer_id,
        repo_scope,
        Some((client_id, client_op_id)),
        &mut op_entry_builder,
    )
}

fn append_generated_op_inner(
    db: &Database,
    doc_id: DocId,
    peer_id: PeerId,
    repo_scope: &str,
    client_ref: Option<(u64, u64)>,
    op_entry_builder: &mut impl FnMut(u64) -> LedgerEntry,
) -> Result<(u64, u64)> {
    let write_txn = db.begin_write()?;
    let next_peer_seq = super::write_direct::next_peer_fact_seq(&write_txn, &peer_id)?;

    let entry = op_entry_builder(next_peer_seq.get());
    if entry.peer_seq != next_peer_seq {
        return Err(anyhow!("Entry sequence mismatch"));
    }
    if entry.origin_peer_id != peer_id {
        return Err(anyhow!("Entry origin peer mismatch"));
    }
    ensure_content_entry(&entry, doc_id)?;
    let new_global_seq_key = super::write_direct::append_op_to_txn(&write_txn, &entry, repo_scope)?;
    let mut client_ops = write_txn.open_table(CLIENT_OP_INDEX)?;
    if let Some((client_id, client_op_id)) = client_ref {
        if client_ops.get((client_id, client_op_id))?.is_some() {
            return Err(anyhow!(
                "Client op already indexed: ({}, {})",
                client_id,
                client_op_id
            ));
        }
        if entry.doc_id.is_none() {
            return Err(anyhow!("client op missing doc id"));
        }
        client_ops.insert((client_id, client_op_id), new_global_seq_key)?;
    }
    drop(client_ops);
    write_txn.commit()?;
    Ok((new_global_seq_key, next_peer_seq.get()))
}

fn ensure_content_entry(entry: &LedgerEntry, expected_doc_id: DocId) -> Result<()> {
    if !matches!(entry.event, LedgerEvent::Content(_)) {
        return Err(anyhow!(
            "Generated content append cannot accept structure events"
        ));
    }
    if entry.doc_id != Some(expected_doc_id) {
        return Err(anyhow!("Generated content entry doc_id mismatch"));
    }
    Ok(())
}
