// crates/core/src/ledger/manager/snapshot_ops.rs
//! # 快照管理
//!
//! 实现 `RepoManager` 的快照保存方法。

use crate::ledger::RepoManager;
use crate::ledger::snapshot;
use crate::models::DocId;
use anyhow::Result;

impl RepoManager {
    /// 保存文档快照 (仅限本地库)
    pub fn save_snapshot(&self, doc_id: DocId, seq: u64, content: &str) -> Result<()> {
        snapshot::save_snapshot(&self.local_db, doc_id, seq, content, self.snapshot_depth)
    }

    /// 读取文档的最新快照 (仅限本地库)
    pub fn load_latest_snapshot(&self, doc_id: DocId) -> Result<Option<(u64, String)>> {
        snapshot::load_latest_snapshot(&self.local_db, doc_id)
    }
}
