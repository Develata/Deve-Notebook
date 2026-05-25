//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
//! Snapshot plus bounded delta payload construction.

use super::{
    confirmed,
    snapshot_delta_guard::{find_delta_chain_issue, issue_summary},
};
use deve_core::models::{DocId, LedgerEntry};
use deve_core::protocol::ConfirmedOp;

pub(super) type SnapshotPayload = (String, u64, Vec<ConfirmedOp>, u64);

pub(super) fn build_snapshot_payload(
    db: &redb::Database,
    doc_id: DocId,
    snapshot_depth: usize,
    repo_scope: &str,
) -> anyhow::Result<SnapshotPayload> {
    ensure_doc_exists(db, doc_id)?;
    let snapshot = deve_core::ledger::snapshot::load_latest_snapshot(db, doc_id)?;
    let has_snapshot = snapshot.is_some();
    let (base_seq, content) = snapshot.unwrap_or((0, String::new()));

    let delta_ops = confirmed::load_doc_ops_after(db, doc_id, base_seq)?;
    let version = delta_ops.last().map(|entry| entry.seq).unwrap_or(base_seq);
    let delta_issue = has_snapshot
        .then(|| find_delta_chain_issue(&content, &delta_ops))
        .flatten();

    if let Some(issue) = delta_issue {
        let path = doc_path_label(db, doc_id);
        tracing::warn!(
            repo_scope,
            doc_id = %doc_id,
            path = %path,
            base_seq,
            version,
            delta_ops = delta_ops.len(),
            issue = %issue_summary(issue),
            "OpenDoc snapshot fallback"
        );
    }
    if !has_snapshot
        || missing_base_snapshot(&content, base_seq, &delta_ops)
        || delta_issue.is_some()
    {
        return rebuild_full_snapshot(db, doc_id, snapshot_depth, repo_scope);
    }

    Ok((content, base_seq, delta_ops, version))
}

fn missing_base_snapshot(content: &str, base_seq: u64, delta_ops: &[ConfirmedOp]) -> bool {
    content.is_empty() && base_seq == 0 && !delta_ops.is_empty()
}

fn rebuild_full_snapshot(
    db: &redb::Database,
    doc_id: DocId,
    snapshot_depth: usize,
    repo_scope: &str,
) -> anyhow::Result<SnapshotPayload> {
    ensure_doc_exists(db, doc_id)?;

    let full_entries = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
    if full_entries.is_empty() {
        return Ok((String::new(), 0, Vec::new(), 0));
    }

    let ops: Vec<LedgerEntry> = full_entries
        .iter()
        .map(|(_, entry)| entry.clone())
        .collect();
    let full_content = deve_core::state::reconstruct_content(&ops);
    let full_version = full_entries.last().map(|(seq, _)| *seq).unwrap_or(0);
    persist_rebuilt_snapshot(
        db,
        doc_id,
        full_version,
        &full_content,
        snapshot_depth,
        repo_scope,
    )?;
    Ok((full_content, full_version, Vec::new(), full_version))
}

fn ensure_doc_exists(db: &redb::Database, doc_id: DocId) -> anyhow::Result<()> {
    if deve_core::ledger::node_meta::file_meta_for_doc(db, doc_id)?.is_none() {
        anyhow::bail!("Document not found: {}", doc_id);
    }
    Ok(())
}

fn doc_path_label(db: &redb::Database, doc_id: DocId) -> String {
    deve_core::ledger::node_meta::path_for_doc(db, doc_id)
        .ok()
        .flatten()
        .unwrap_or_else(|| "<unknown>".into())
}

fn persist_rebuilt_snapshot(
    db: &redb::Database,
    doc_id: DocId,
    version: u64,
    content: &str,
    snapshot_depth: usize,
    repo_scope: &str,
) -> anyhow::Result<()> {
    let verified = deve_core::ledger::snapshot::verify_snapshot_consistency(
        db, doc_id, version, content, true,
    )?;
    if !verified {
        let path = doc_path_label(db, doc_id);
        tracing::warn!(
            repo_scope,
            doc_id = %doc_id,
            path = %path,
            version,
            "OpenDoc snapshot rebuild skipped persistence"
        );
        return Ok(());
    }
    deve_core::ledger::snapshot::save_snapshot(db, doc_id, version, content, snapshot_depth)
}
