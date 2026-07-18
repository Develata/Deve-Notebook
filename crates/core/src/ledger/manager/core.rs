//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-selector-resolution-contract
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/index#repo-runtime-layout
//!
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::node_meta;
use crate::ledger::{init, range};
use crate::models::{LedgerEntry, NodeId, NodeMeta, PeerFactSeq, PeerId, RepoId};

impl RepoManager {
    pub(crate) fn lock_shadow_merge(&self) -> Result<std::sync::MutexGuard<'_, ()>> {
        self.shadow_merge_guard
            .lock()
            .map_err(|_| anyhow!("shadow/merge authority guard poisoned"))
    }

    /// 初始化仓库管理器
    ///
    /// 详细文档见 `init` 模块。
    pub fn init(
        ledger_dir: impl AsRef<Path>,
        snapshot_depth: usize,
        repo_name: Option<&str>,
        repo_url: Option<&str>,
    ) -> Result<Self> {
        init::init(ledger_dir, snapshot_depth, repo_name, repo_url)
    }

    pub fn init_with_options(
        ledger_dir: impl AsRef<Path>,
        snapshot_depth: usize,
        repo_name: Option<&str>,
        options: init::RepoInitOptions,
    ) -> Result<Self> {
        init::init_with_options(ledger_dir, snapshot_depth, repo_name, options)
    }

    pub fn init_existing_for_repo_id(
        ledger_dir: impl AsRef<Path>,
        snapshot_depth: usize,
        repo_id: RepoId,
    ) -> Result<Self> {
        let ledger_dir = ledger_dir.as_ref();
        let local_dir =
            Self::checked_local_dir_for(ledger_dir, "opening existing local repo by RepoId")?;
        for (path, _stem) in redb_repo_entries(&local_dir, "opening existing local repo by RepoId")?
        {
            let Some(info) = Self::read_local_repo_info_from_path(&path)? else {
                continue;
            };
            if info.uuid == repo_id {
                return init::init_with_options(
                    ledger_dir,
                    snapshot_depth,
                    Some(&info.name),
                    init::RepoInitOptions {
                        repo_id: Some(repo_id),
                        repo_url: info.url,
                    },
                );
            }
        }
        Err(anyhow!("Local repo not found for UUID {}", repo_id))
    }

    /// 执行闭包于指定的本地仓库 (按名称)
    ///
    /// * `repo_name`: 仓库名称 (e.g. "default", "wiki").
    /// * `f`: 接收 &Database 的闭包.
    pub fn run_on_local_repo<F, R>(&self, repo_name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&redb::Database) -> Result<R>,
    {
        let selector = repo_name;
        if let Some(stem) = self.resolve_local_repo_stem(selector)? {
            return self.run_on_local_repo_stem(&stem, f);
        }
        self.refresh_local_repo_catalog()?;
        if let Some(stem) = self.resolve_local_repo_stem(selector)? {
            return self.run_on_local_repo_stem(&stem, f);
        }
        Err(anyhow!("Repository not found: {}", selector))
    }

    /// 获取主本地仓库的规范执行 selector（物理 RepoId stem）。
    pub fn local_repo_name(&self) -> &str {
        &self.local_repo_name
    }

    /// 列出指定本地仓库的文档
    pub fn list_local_docs(
        &self,
        repo_name: Option<&str>,
    ) -> Result<Vec<(crate::models::DocId, String)>> {
        let name = repo_name.unwrap_or(&self.local_repo_name);
        self.run_on_local_repo(name, node_meta::list_file_docs)
    }

    /// 列出指定本地仓库的节点
    pub fn list_local_nodes(&self, repo_name: Option<&str>) -> Result<Vec<(NodeId, NodeMeta)>> {
        let name = repo_name.unwrap_or(&self.local_repo_name);
        self.run_on_local_repo(name, node_meta::list_nodes)
    }

    /// 获取账本目录路径
    pub fn ledger_dir(&self) -> &Path {
        &self.ledger_dir
    }

    pub fn local_peer_id(&self) -> &crate::models::PeerId {
        &self.local_peer_id
    }

    pub fn snapshot_depth(&self) -> usize {
        self.snapshot_depth
    }

    /// 获取影子库目录路径
    pub fn remotes_dir(&self) -> PathBuf {
        self.ledger_dir.join("remotes")
    }

    /// 获取本地数据库的只读事务 (用于高级查询)
    pub fn local_db_read_txn(&self) -> Result<redb::ReadTransaction> {
        Self::validate_local_repo_schema(self.local_db.as_ref())?;
        Ok(self.local_db.begin_read()?)
    }

    /// 获取本地库指定序列号范围的操作 (用于 P2P 同步增量推送)
    pub fn get_local_ops_in_range(
        &self,
        repo_id: &RepoId,
        peer_id: &PeerId,
        start_seq: PeerFactSeq,
        end_seq: PeerFactSeq,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.run_on_local_repo_id(repo_id, |db| {
            range::get_peer_ops_in_range(db, peer_id, start_seq, end_seq)
        })
    }

    pub fn get_local_peer_waterline(&self, repo_id: &RepoId) -> Result<PeerFactSeq> {
        self.run_on_local_repo_id(repo_id, |db| {
            range::get_peer_waterline(db, self.local_peer_id())
        })
    }
}

#[cfg(test)]
mod tests;
