use anyhow::Result;

use super::{RepoInfo, RepoManager};
use crate::models::{PeerId, RepoId};

impl RepoManager {
    /// Invariants:
    /// - `ensure_shadow_repo_binding` 只负责建立 repo-scoped shadow 实例，不得猜测远端 metadata。
    /// - 远端 `RepoInfo` 只能来自 shadow 自身显式元数据或后续受控 repair，不得借本地同 UUID 仓库推断。
    /// - 当调用者显式给出 `repo_id` 时，可写入 `uuid/name=uuid/url=None` 的最小元数据以维持 shadow catalog 自洽。
    pub fn ensure_shadow_repo_binding(&self, peer_id: &PeerId, repo_id: RepoId) -> Result<()> {
        self.ensure_shadow_repo_info(
            peer_id,
            &RepoInfo {
                uuid: repo_id,
                name: repo_id.to_string(),
                url: None,
            },
        )
    }
}
