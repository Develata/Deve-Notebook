//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 05_diff_logic#merge-contract
//!
//! Validation and reconstruction helpers for checkpoint-backed peer merge writes.

use crate::codec;
use crate::ledger::merge::{MergeBaseCheckpoint, MergePreflight};
use crate::ledger::schema::{LEDGER_OPS, MERGE_BASE_CHECKPOINT, PEER_FACT_OPS};
use crate::ledger::{ops, range};
use crate::models::{
    DocId, LedgerEntry, MergeResolution, PeerFactSeq, PeerId, deserialize_ledger_entry,
};
use crate::{security, state};
use anyhow::{Context, Result, anyhow, bail};
use redb::ReadableTable;

pub(super) fn stable_doc_snapshot(
    db: &redb::Database,
    peer_id: &PeerId,
    doc_id: DocId,
) -> Result<(PeerFactSeq, Vec<LedgerEntry>)> {
    let before = range::get_peer_waterline(db, peer_id)?;
    let entries = ops::get_ops_from_db(db, doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    let after = range::get_peer_waterline(db, peer_id)?;
    if before != after {
        bail!(
            "merge_snapshot_drift: source={} before={} after={}",
            peer_id,
            before,
            after
        );
    }
    Ok((after, entries))
}

pub(super) fn ensure_source_entries(
    source_peer_id: &PeerId,
    entries: &[LedgerEntry],
) -> Result<()> {
    if let Some(entry) = entries
        .iter()
        .find(|entry| entry.origin_peer_id != *source_peer_id)
    {
        bail!(
            "merge_source_origin_mismatch: expected={} observed={} seq={}",
            source_peer_id,
            entry.origin_peer_id,
            entry.peer_seq
        );
    }
    Ok(())
}

pub(super) fn reconstruct(entries: &[LedgerEntry]) -> String {
    state::reconstruct_content(entries)
}

pub(super) fn reconstruct_at(entries: &[LedgerEntry], waterline: PeerFactSeq) -> String {
    let visible = entries
        .iter()
        .filter(|entry| entry.peer_seq <= waterline)
        .cloned()
        .collect::<Vec<_>>();
    state::reconstruct_content(&visible)
}

pub(super) fn hash_content(content: &str) -> [u8; 32] {
    security::hashing::sha256_bytes(content.as_bytes())
}

pub(super) fn validate_resolution(
    preflight: &MergePreflight,
    target_content: &str,
    resolution: MergeResolution,
) -> Result<()> {
    if preflight.establish_equal {
        if resolution != MergeResolution::EstablishEqual
            || preflight.local_content != preflight.source_content
            || target_content != preflight.local_content
        {
            bail!("invalid initial merge baseline resolution");
        }
        return Ok(());
    }
    match resolution {
        MergeResolution::EstablishEqual => {
            bail!("EstablishEqual is only valid without an existing checkpoint")
        }
        MergeResolution::Auto => {
            if preflight.automatic_result.as_deref() != Some(target_content) {
                bail!("automatic merge target does not match the evaluated result");
            }
        }
        MergeResolution::AcceptCurrent => {
            if preflight.automatic_result.is_some() || target_content != preflight.local_content {
                bail!("AcceptCurrent requires a conflict and the evaluated local content");
            }
        }
        MergeResolution::AcceptIncoming => {
            if preflight.automatic_result.is_some() || target_content != preflight.source_content {
                bail!("AcceptIncoming requires a conflict and the evaluated source content");
            }
        }
        MergeResolution::AcceptBoth => {
            if preflight.automatic_result.is_some() {
                bail!("AcceptBoth requires an evaluated conflict");
            }
        }
    }
    Ok(())
}

pub(super) fn ensure_checkpoint_generation(
    write_txn: &redb::WriteTransaction,
    source_peer_id: &PeerId,
    doc_id: DocId,
    expected_anchor_global_seq: Option<u64>,
) -> Result<()> {
    let table = write_txn.open_table(MERGE_BASE_CHECKPOINT)?;
    let current = table.get((source_peer_id.as_str(), doc_id.as_u128()))?;
    let observed = current
        .as_ref()
        .map(|bytes| codec::decode::<MergeBaseCheckpoint>(bytes.value()))
        .transpose()?
        .map(|checkpoint| checkpoint.anchor_global_seq);
    if observed != expected_anchor_global_seq {
        bail!(
            "merge_checkpoint_drift: source={} doc={} expected={:?} observed={:?}",
            source_peer_id,
            doc_id,
            expected_anchor_global_seq,
            observed
        );
    }
    Ok(())
}

pub(super) fn validate_anchor_reference(
    db: &redb::Database,
    local_peer_id: &PeerId,
    checkpoint: &MergeBaseCheckpoint,
) -> Result<()> {
    let read = db.begin_read()?;
    let peer_ops = read.open_table(PEER_FACT_OPS)?;
    let indexed_global = peer_ops
        .get((
            local_peer_id.as_str(),
            checkpoint.local_anchor_peer_seq.get(),
        ))?
        .ok_or_else(|| anyhow!("merge_checkpoint_anchor_index_missing"))?
        .value();
    if indexed_global != checkpoint.anchor_global_seq {
        bail!(
            "merge_checkpoint_anchor_index_mismatch: expected={} observed={}",
            checkpoint.anchor_global_seq,
            indexed_global
        );
    }
    let ledger = read.open_table(LEDGER_OPS)?;
    let bytes = ledger
        .get(checkpoint.anchor_global_seq)?
        .ok_or_else(|| anyhow!("merge_checkpoint_anchor_missing"))?;
    let entry = deserialize_ledger_entry(bytes.value())
        .with_context(|| "failed to decode merge checkpoint anchor")?;
    let anchor = entry
        .merge_anchor()
        .ok_or_else(|| anyhow!("merge_checkpoint_anchor_event_mismatch"))?;
    if entry.origin_peer_id != *local_peer_id
        || entry.peer_seq != checkpoint.local_anchor_peer_seq
        || entry.doc_id != Some(checkpoint.doc_id)
        || entry.actor.as_str() != "merge"
        || anchor.source_peer_id != checkpoint.source_peer_id
        || anchor.source_waterline != checkpoint.source_peer_seq
        || anchor.source_state_hash != checkpoint.source_state_hash
        || anchor.result_hash != checkpoint.result_hash
    {
        bail!("merge_checkpoint_anchor_payload_mismatch");
    }
    Ok(())
}
