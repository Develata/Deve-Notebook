// crates/core/src/ledger/manager/ops_ops.rs
//! # 操作日志追加/读取
//!
//! 实现 `RepoManager` 的操作追加和读取方法。

use crate::ledger::RepoManager;
use crate::ledger::ops;
use crate::models::{DocId, LedgerEntry, PeerId, RepoType};
use anyhow::Result;

impl RepoManager {
    /// 追加操作到本地库 (Store B)
    ///
    /// **权限**: Local Write Only - 仅接受本地用户的操作。
    pub fn append_local_op(&self, entry: &LedgerEntry) -> Result<u64> {
        ops::append_op_to_db(&self.local_db, entry)
    }

    /// 原子生成序号并追加操作 (推荐用于本地编辑)
    ///
    /// 自动计算下一个 Local Sequence，避免竞态条件。
    /// 返回: (GlobalSeq, LocalSeq)
    pub fn append_generated_op(
        &self,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        ops::append_generated_op(&self.local_db, doc_id, peer_id, op_entry_builder)
    }

    /// 从指定仓库读取操作
    pub fn get_ops(&self, repo_type: &RepoType, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        match repo_type {
            RepoType::Local(_) => ops::get_ops_from_db(&self.local_db, doc_id),
            RepoType::Remote(peer_id, repo_id) => {
                self.ensure_shadow_db(peer_id, repo_id)?;
                let dbs = self.shadow_dbs.read().unwrap();
                let peer_repos = dbs
                    .get(peer_id)
                    .ok_or_else(|| anyhow::anyhow!("未找到 Peer 的影子库集合: {}", peer_id))?;
                let db = peer_repos.get(repo_id).ok_or_else(|| {
                    anyhow::anyhow!("未找到指定 Repo 的影子库: {}/{}", peer_id, repo_id)
                })?;
                ops::get_ops_from_db(db, doc_id)
            }
        }
    }

    /// 从本地库读取操作（便捷方法）
    pub fn get_local_ops(&self, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        self.get_ops(&RepoType::Local(uuid::Uuid::nil()), doc_id)
    }
}
