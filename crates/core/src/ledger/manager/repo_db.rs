use crate::ledger::RepoManager;
use crate::models::{PeerId, RepoId, RepoType};
use anyhow::{Result, anyhow};
use redb::Database;

impl RepoManager {
    /// Invariants:
    /// - `RepoType::Local(repo_id)` 必须先解析到真实 local repo instance。
    /// - 后续读路径不得在忽略 `repo_id` 的前提下回退到 `local_db`。
    pub fn run_on_repo_db<F, R>(&self, repo_type: &RepoType, f: F) -> Result<R>
    where
        F: FnOnce(&Database) -> Result<R>,
    {
        match repo_type {
            RepoType::Local(repo_id) => self.run_on_local_repo_id(repo_id, f),
            RepoType::Remote(peer_id, repo_id) => self.run_on_shadow_repo(peer_id, repo_id, f),
        }
    }

    pub(crate) fn run_on_local_repo_id<F, R>(&self, repo_id: &RepoId, f: F) -> Result<R>
    where
        F: FnOnce(&Database) -> Result<R>,
    {
        let repo_name = self
            .find_local_repo_name_by_id(*repo_id)?
            .ok_or_else(|| anyhow!("Local repo not found for UUID {}", repo_id))?;
        self.run_on_local_repo(&repo_name, f)
    }

    pub(crate) fn run_on_shadow_repo<F, R>(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        f: F,
    ) -> Result<R>
    where
        F: FnOnce(&Database) -> Result<R>,
    {
        self.ensure_shadow_db(peer_id, repo_id)?;
        let dbs = self.read_shadow_dbs()?;
        let peer_repos = dbs
            .get(peer_id)
            .ok_or_else(|| anyhow!("未找到 Peer 的影子库集合: {}", peer_id))?;
        let db = peer_repos
            .get(repo_id)
            .ok_or_else(|| anyhow!("未找到指定 Repo 的影子库: {}/{}", peer_id, repo_id))?;
        f(db.as_ref())
    }

    /// Invariants:
    /// - 远端只读路径必须先按 `PeerId + RepoId` 锁定 shadow DB。
    /// - 调用方不得绕过 `RepoId` 再回退到名称匹配。
    pub fn run_on_shadow_repo_by_id<F, R>(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        f: F,
    ) -> Result<R>
    where
        F: FnOnce(&Database) -> Result<R>,
    {
        self.run_on_shadow_repo(peer_id, repo_id, f)
    }
}
