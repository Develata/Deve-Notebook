use anyhow::Result;

use super::RepoManager;
use crate::models::{PeerId, RepoId};

impl RepoManager {
    /// Invariants:
    /// - 远端影子库元数据优先对齐同 `repo_id` 的本地 `RepoInfo`。
    /// - 若本地没有该 repo 的元数据，仍必须至少建立 repo-scoped 影子库。
    pub fn ensure_shadow_repo_binding(&self, peer_id: &PeerId, repo_id: RepoId) -> Result<()> {
        if let Some(info) = self.get_local_repo_info_by_id(repo_id)? {
            return self.ensure_shadow_repo_info(peer_id, &info);
        }
        self.ensure_shadow_db(peer_id, &repo_id)
    }
}
