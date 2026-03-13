use super::confirmed;
use deve_core::models::{DocId, LedgerEntry};
use deve_core::protocol::ConfirmedOp;

pub(super) type SnapshotPayload = (String, u64, Vec<ConfirmedOp>, u64);

pub(super) fn build_snapshot_payload(
    db: &redb::Database,
    doc_id: DocId,
    snapshot_depth: usize,
) -> anyhow::Result<SnapshotPayload> {
    let snapshot = deve_core::ledger::snapshot::load_latest_snapshot(db, doc_id)?;
    let has_snapshot = snapshot.is_some();
    let (base_seq, content) = snapshot.unwrap_or((0, String::new()));

    let delta_ops = confirmed::load_doc_ops_after(db, doc_id, base_seq)?;
    let version = delta_ops.last().map(|entry| entry.seq).unwrap_or(base_seq);

    if !has_snapshot || (content.is_empty() && base_seq == 0 && !delta_ops.is_empty()) {
        return rebuild_full_snapshot(db, doc_id, snapshot_depth);
    }

    Ok((content, base_seq, delta_ops, version))
}

fn rebuild_full_snapshot(
    db: &redb::Database,
    doc_id: DocId,
    snapshot_depth: usize,
) -> anyhow::Result<SnapshotPayload> {
    if deve_core::ledger::node_meta::file_meta_for_doc(db, doc_id)?.is_none() {
        anyhow::bail!("Document not found: {}", doc_id);
    }

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
    let _ = deve_core::ledger::snapshot::save_snapshot(
        db,
        doc_id,
        full_version,
        &full_content,
        snapshot_depth,
    );
    Ok((full_content, full_version, Vec::new(), full_version))
}
