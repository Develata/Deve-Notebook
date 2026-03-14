use anyhow::Result;

use super::RepoManager;
use crate::models::{PeerId, RepoId};

impl RepoManager {
    /// Invariants:
    /// - `ensure_shadow_repo_binding` 只负责建立 repo-scoped shadow 实例，不得猜测远端 metadata。
    /// - 远端 `RepoInfo` 只能来自 shadow 自身显式元数据或后续受控 repair，不得借本地同 UUID 仓库推断。
    pub fn ensure_shadow_repo_binding(&self, peer_id: &PeerId, repo_id: RepoId) -> Result<()> {
        self.ensure_shadow_db(peer_id, &repo_id)
    }
}
