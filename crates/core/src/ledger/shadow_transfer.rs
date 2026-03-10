use anyhow::Result;

use super::RepoManager;
use super::ops;
use super::range;
use crate::ledger::manager::structure_projection;
use crate::models::{LedgerEntry, LedgerEvent, PeerId, RepoId};

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
        self.run_on_shadow_repo(peer_id, repo_id, |db| {
            let seq = ops::append_op_to_db(db, entry)?;
            if let LedgerEvent::Structure(op) = &entry.event {
                structure_projection::apply(db, op)?;
            }
            Ok(seq)
        })
    }
}
