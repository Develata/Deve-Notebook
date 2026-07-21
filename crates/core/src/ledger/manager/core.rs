//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-selector-resolution-contract
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/index#repo-runtime-layout
//!
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

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
        init::init_existing_for_repo_id(ledger_dir, snapshot_depth, repo_id)
    }

    /// Composes a host runtime with no local repo authority mounted.
    ///
    /// This is the legal `NoScope` bootstrap path. It creates only host-owned
    /// directories and identity; it never creates or opens a local Redb.
    pub fn init_empty_host(ledger_dir: impl AsRef<Path>, snapshot_depth: usize) -> Result<Self> {
        init::init_empty_host(ledger_dir, snapshot_depth)
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

    /// Returns the bootstrap repo selector, if this process started with one.
    ///
    /// This is retained for bootstrap/test compatibility only. Product paths
    /// must use an explicit RepoId or [`Self::current_local_repo_name`], because
    /// the bootstrap member can later be removed.
    pub fn local_repo_name(&self) -> &str {
        self.local_authority.primary_repo_name().unwrap_or("")
    }

    /// Resolves the current implicit local selector from durable membership.
    ///
    /// Product paths should prefer an explicit RepoId. This bounded default is
    /// retained for legacy host APIs, but it never returns a removed bootstrap
    /// anchor: after lifecycle transitions it selects the sole remaining repo,
    /// keeps a still-live bootstrap member, or fails closed on zero/ambiguity.
    pub fn current_local_repo_name(&self) -> Result<String> {
        let summaries = self.list_cataloged_local_repo_summaries()?;
        if let Some(primary) = self.local_authority.primary_repo_id()
            && summaries.iter().any(|summary| summary.repo_id == primary)
        {
            return Ok(primary.to_string());
        }
        match summaries.as_slice() {
            [summary] => Ok(summary.execution_name.clone()),
            [] => Err(anyhow!("no local repository is selected")),
            _ => Err(anyhow!(
                "multiple local repositories exist without an explicit RepoId selector"
            )),
        }
    }

    /// 列出指定本地仓库的文档
    pub fn list_local_docs(
        &self,
        repo_name: Option<&str>,
    ) -> Result<Vec<(crate::models::DocId, String)>> {
        let default_name;
        let name = match repo_name {
            Some(name) => name,
            None => {
                default_name = self.current_local_repo_name()?;
                &default_name
            }
        };
        self.run_on_local_repo(name, node_meta::list_file_docs)
    }

    /// 列出指定本地仓库的节点
    pub fn list_local_nodes(&self, repo_name: Option<&str>) -> Result<Vec<(NodeId, NodeMeta)>> {
        let default_name;
        let name = match repo_name {
            Some(name) => name,
            None => {
                default_name = self.current_local_repo_name()?;
                &default_name
            }
        };
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

    pub(crate) fn run_on_primary_local_repo<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&redb::Database) -> Result<R>,
    {
        let repo_name = self.current_local_repo_name()?;
        self.run_on_local_repo(&repo_name, f)
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
