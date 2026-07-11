//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/authority#facts-partition
//!   - 04_repository#tree-projection-contract
//!
use anyhow::Result;

use super::RepoManager;
use super::ops;
use super::range;
use crate::ledger::manager::structure_projection;
use crate::ledger::schema::{LEDGER_OPS, PEER_FACT_OPS, PEER_FACT_SEQ};
use crate::models::{
    LedgerEntry, LedgerEvent, PeerFactSeq, PeerId, RepoId, deserialize_ledger_entry,
};
use redb::{ReadableTable, WriteTransaction};

pub(crate) enum ShadowPayload<'a> {
    Ops(&'a [LedgerEntry]),
    Snapshot(&'a [LedgerEntry]),
}

struct ShadowAppendOutcome {
    last_global_seq: u64,
    max_remote_seq: u64,
}

impl RepoManager {
    pub fn get_shadow_max_seq(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<PeerFactSeq> {
        self.run_on_shadow_repo(peer_id, repo_id, |db| {
            range::get_peer_waterline(db, peer_id)
        })
    }

    pub fn get_shadow_ops_in_range(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        start_seq: PeerFactSeq,
        end_seq: PeerFactSeq,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.run_on_shadow_repo(peer_id, repo_id, |db| {
            range::get_peer_ops_in_range(db, peer_id, start_seq, end_seq)
        })
    }

    /// Invariants:
    /// - Remote shadow 的权威输入仍然是 Ledger append。
    /// - 结构事实进入影子库后，必须同步更新只读 projection。
    pub fn append_remote_op(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        entry: &LedgerEntry,
    ) -> Result<u64> {
        let entries = std::slice::from_ref(entry);
        let payloads = [ShadowPayload::Ops(entries)];
        let outcome = self.apply_remote_payloads_internal(peer_id, repo_id, &payloads)?;
        Ok(outcome.last_global_seq)
    }

    pub fn append_remote_ops(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        entries: &[LedgerEntry],
    ) -> Result<u64> {
        let payloads = [ShadowPayload::Ops(entries)];
        self.apply_remote_payloads(peer_id, repo_id, &payloads)
    }

    pub fn replace_shadow_repo_ops(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        entries: &[LedgerEntry],
    ) -> Result<u64> {
        let payloads = [ShadowPayload::Snapshot(entries)];
        self.apply_remote_payloads(peer_id, repo_id, &payloads)
    }

    pub(crate) fn apply_remote_payloads(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        payloads: &[ShadowPayload<'_>],
    ) -> Result<u64> {
        let outcome = self.apply_remote_payloads_internal(peer_id, repo_id, payloads)?;
        Ok(outcome.max_remote_seq)
    }

    fn apply_remote_payloads_internal(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        payloads: &[ShadowPayload<'_>],
    ) -> Result<ShadowAppendOutcome> {
        let _guard = self.lock_shadow_merge()?;
        let repo_scope = ops::shadow_repo_scope(peer_id, repo_id);
        for payload in payloads {
            let entries = match payload {
                ShadowPayload::Ops(entries) | ShadowPayload::Snapshot(entries) => *entries,
            };
            if let Some(entry) = entries
                .iter()
                .find(|entry| entry.origin_peer_id != *peer_id)
            {
                anyhow::bail!(
                    "Remote shadow source mismatch: target={}, entry_origin={}, peer_seq={}",
                    peer_id,
                    entry.origin_peer_id,
                    entry.peer_seq
                );
            }
        }
        self.run_on_shadow_repo(peer_id, repo_id, |db| {
            let write_txn = db.begin_write()?;
            let mut stored_waterline = peer_waterline_txn(&write_txn, peer_id)?;
            let mut outcome = ShadowAppendOutcome {
                last_global_seq: 0,
                max_remote_seq: stored_waterline.get(),
            };
            let mut saw_ops = false;
            for payload in payloads {
                match payload {
                    ShadowPayload::Ops(entries) => {
                        saw_ops = true;
                        merge_outcome(
                            &mut outcome,
                            apply_remote_entries_txn(
                                &write_txn,
                                peer_id,
                                &repo_scope,
                                entries,
                                &mut stored_waterline,
                            )?,
                        );
                    }
                    ShadowPayload::Snapshot(entries) => {
                        if saw_ops {
                            anyhow::bail!("snapshot must precede incremental facts in one batch");
                        }
                        merge_outcome(
                            &mut outcome,
                            apply_remote_snapshot_txn(
                                &write_txn,
                                peer_id,
                                &repo_scope,
                                entries,
                                &mut stored_waterline,
                            )?,
                        );
                    }
                }
            }
            write_txn.commit()?;
            Ok(outcome)
        })
    }
}

fn apply_remote_entries_txn(
    write_txn: &WriteTransaction,
    peer_id: &PeerId,
    repo_scope: &str,
    entries: &[LedgerEntry],
    stored_waterline: &mut PeerFactSeq,
) -> Result<ShadowAppendOutcome> {
    let mut outcome = ShadowAppendOutcome {
        last_global_seq: 0,
        max_remote_seq: stored_waterline.get(),
    };
    validate_contiguous_entries(entries, None)?;
    for entry in entries {
        if entry.peer_seq <= *stored_waterline {
            ensure_confirmed_entry_matches(write_txn, peer_id, entry)?;
            continue;
        }
        let expected = stored_waterline
            .next()
            .ok_or_else(|| anyhow::anyhow!("PeerFactSeq overflow after {stored_waterline}"))?;
        if entry.peer_seq != expected {
            anyhow::bail!(
                "sequence_gap: source={} expected={} observed={}",
                peer_id,
                expected,
                entry.peer_seq
            );
        }
        outcome.last_global_seq = ops::append_op_to_txn(write_txn, entry, repo_scope)?;
        outcome.max_remote_seq = outcome.max_remote_seq.max(entry.peer_seq.get());
        *stored_waterline = entry.peer_seq;
        if let LedgerEvent::Structure(op) = &entry.event {
            structure_projection::apply_in_txn(write_txn, op)?;
        }
    }
    Ok(outcome)
}

fn apply_remote_snapshot_txn(
    write_txn: &WriteTransaction,
    peer_id: &PeerId,
    repo_scope: &str,
    entries: &[LedgerEntry],
    stored_waterline: &mut PeerFactSeq,
) -> Result<ShadowAppendOutcome> {
    validate_contiguous_entries(entries, Some(PeerFactSeq::ONE))?;
    let incoming_waterline = entries
        .last()
        .map(|entry| entry.peer_seq)
        .unwrap_or(PeerFactSeq::ZERO);
    for entry in entries
        .iter()
        .take_while(|entry| entry.peer_seq <= *stored_waterline)
    {
        ensure_confirmed_entry_matches(write_txn, peer_id, entry)?;
    }
    if incoming_waterline <= *stored_waterline {
        return Ok(ShadowAppendOutcome {
            last_global_seq: 0,
            max_remote_seq: stored_waterline.get(),
        });
    }

    RepoManager::reset_shadow_repo_txn(write_txn)?;
    *stored_waterline = PeerFactSeq::ZERO;
    apply_remote_entries_txn(write_txn, peer_id, repo_scope, entries, stored_waterline)
}

fn validate_contiguous_entries(
    entries: &[LedgerEntry],
    required_start: Option<PeerFactSeq>,
) -> Result<()> {
    let Some(first) = entries.first() else {
        return Ok(());
    };
    if first.peer_seq == PeerFactSeq::ZERO {
        anyhow::bail!("remote fact sequence must be positive");
    }
    if let Some(required_start) = required_start
        && first.peer_seq != required_start
    {
        anyhow::bail!(
            "sequence_gap: snapshot expected={} observed={}",
            required_start,
            first.peer_seq
        );
    }
    let mut expected = first.peer_seq;
    for entry in entries {
        if entry.peer_seq != expected {
            anyhow::bail!(
                "non-contiguous remote ops: expected seq {}, received {}",
                expected,
                entry.peer_seq
            );
        }
        expected = entry
            .peer_seq
            .next()
            .ok_or_else(|| anyhow::anyhow!("PeerFactSeq overflow after {}", entry.peer_seq))?;
    }
    Ok(())
}

fn peer_waterline_txn(write_txn: &WriteTransaction, peer_id: &PeerId) -> Result<PeerFactSeq> {
    Ok(write_txn
        .open_table(PEER_FACT_SEQ)?
        .get(peer_id.as_str())?
        .map(|value| PeerFactSeq::new(value.value()))
        .unwrap_or(PeerFactSeq::ZERO))
}

fn ensure_confirmed_entry_matches(
    write_txn: &WriteTransaction,
    peer_id: &PeerId,
    incoming: &LedgerEntry,
) -> Result<()> {
    let global_seq = write_txn
        .open_table(PEER_FACT_OPS)?
        .get((peer_id.as_str(), incoming.peer_seq.get()))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "sequence_gap: source={} confirmed peer_seq={} is missing",
                peer_id,
                incoming.peer_seq
            )
        })?
        .value();
    let ledger_ops = write_txn.open_table(LEDGER_OPS)?;
    let bytes = ledger_ops
        .get(global_seq)?
        .ok_or_else(|| anyhow::anyhow!("confirmed GlobalSeq {global_seq} is missing"))?;
    let confirmed = deserialize_ledger_entry(bytes.value())?;
    if confirmed != *incoming {
        anyhow::bail!(
            "sequence_conflict: source={} peer_seq={} confirmed fact differs",
            peer_id,
            incoming.peer_seq
        );
    }
    Ok(())
}

fn merge_outcome(target: &mut ShadowAppendOutcome, source: ShadowAppendOutcome) {
    target.last_global_seq = source.last_global_seq.max(target.last_global_seq);
    target.max_remote_seq = source.max_remote_seq.max(target.max_remote_seq);
}
