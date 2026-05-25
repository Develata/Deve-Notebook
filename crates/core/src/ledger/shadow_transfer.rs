//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage#facts-partition
//!   - 04_repository#tree-projection-contract
//!
use anyhow::Result;

use super::RepoManager;
use super::ops;
use super::range;
use crate::ledger::manager::structure_projection;
use crate::models::{LedgerEntry, LedgerEvent, PeerId, RepoId};
use redb::WriteTransaction;

pub(crate) enum ShadowPayload<'a> {
    Ops(&'a [LedgerEntry]),
    Snapshot(&'a [LedgerEntry]),
}

struct ShadowAppendOutcome {
    last_global_seq: u64,
    max_remote_seq: u64,
}

impl RepoManager {
    pub fn get_shadow_max_seq(&self, peer_id: &PeerId, repo_id: &RepoId) -> Result<u64> {
        self.run_on_shadow_repo(peer_id, repo_id, range::get_max_seq)
    }

    pub fn get_shadow_ops_in_range(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        start_seq: u64,
        end_seq: u64,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.run_on_shadow_repo(peer_id, repo_id, |db| {
            range::get_ops_in_range(db, start_seq, end_seq)
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
        let repo_scope = ops::shadow_repo_scope(peer_id, repo_id);
        self.run_on_shadow_repo(peer_id, repo_id, |db| {
            let write_txn = db.begin_write()?;
            let mut outcome = ShadowAppendOutcome {
                last_global_seq: 0,
                max_remote_seq: 0,
            };
            for payload in payloads {
                match payload {
                    ShadowPayload::Ops(entries) => {
                        merge_outcome(
                            &mut outcome,
                            append_remote_entries_txn(&write_txn, &repo_scope, entries)?,
                        );
                    }
                    ShadowPayload::Snapshot(entries) => {
                        RepoManager::reset_shadow_repo_txn(&write_txn)?;
                        outcome = append_remote_entries_txn(&write_txn, &repo_scope, entries)?;
                    }
                }
            }
            write_txn.commit()?;
            Ok(outcome)
        })
    }
}

fn append_remote_entries_txn(
    write_txn: &WriteTransaction,
    repo_scope: &str,
    entries: &[LedgerEntry],
) -> Result<ShadowAppendOutcome> {
    let mut outcome = ShadowAppendOutcome {
        last_global_seq: 0,
        max_remote_seq: 0,
    };
    for entry in entries {
        outcome.last_global_seq = ops::append_op_to_txn(write_txn, entry, repo_scope)?;
        outcome.max_remote_seq = outcome.max_remote_seq.max(entry.seq);
        if let LedgerEvent::Structure(op) = &entry.event {
            structure_projection::apply_in_txn(write_txn, op)?;
        }
    }
    Ok(outcome)
}

fn merge_outcome(target: &mut ShadowAppendOutcome, source: ShadowAppendOutcome) {
    target.last_global_seq = source.last_global_seq.max(target.last_global_seq);
    target.max_remote_seq = source.max_remote_seq.max(target.max_remote_seq);
}
