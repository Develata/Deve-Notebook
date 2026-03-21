use anyhow::{Context, Result, anyhow};
use redb::Database;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};

use crate::ledger::database::cached_database;
use crate::ledger::manager::repo_catalog_entries::redb_repo_entries;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::node_meta;
use crate::ledger::{init, range, source_control};
use crate::models::{LedgerEntry, NodeId, NodeMeta, RepoId};

impl RepoManager {
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

    /// 设置 Vault 根目录并强制校验本地 catalog。
    ///
    /// Invariants:
    /// - 生产态在挂载 Vault 后，必须立即暴露本地 repo catalog 损坏。
    /// - 测试辅助可继续使用 `set_vault_root` 的宽松包装。
    pub fn set_vault_root_checked(&mut self, root: impl AsRef<Path>) -> Result<()> {
        let previous_root = self.vault_root.clone();
        self.vault_root = Some(root.as_ref().to_path_buf());
        if let Err(err) = self.refresh_local_repo_catalog() {
            self.vault_root = previous_root;
            return Err(err);
        }
        Ok(())
    }

    /// 设置 Vault 根目录 (用于 commit 时读取磁盘文件)
    pub fn set_vault_root(&mut self, root: impl AsRef<Path>) {
        let previous_root = self.vault_root.clone();
        if let Err(error) = self.set_vault_root_checked(root) {
            self.vault_root = previous_root;
            tracing::warn!("Failed to repair local repo catalog after mounting vault: {error}");
        }
    }

    /// 执行闭包于指定的本地仓库 (按名称)
    ///
    /// * `repo_name`: 仓库名称 (e.g. "default", "wiki").
    /// * `f`: 接收 &Database 的闭包.
    pub fn run_on_local_repo<F, R>(&self, repo_name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&redb::Database) -> Result<R>,
    {
        let selector = repo_name.trim_end_matches(".redb");
        if let Some(stem) = self.resolve_local_repo_stem(selector)? {
            return self.run_on_local_repo_stem(&stem, f);
        }
        self.refresh_local_repo_catalog()?;
        if let Some(stem) = self.resolve_local_repo_stem(selector)? {
            return self.run_on_local_repo_stem(&stem, f);
        }
        Err(anyhow!("Repository not found: {}", selector))
    }

    /// 获取主仓库名称
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

    /// 获取影子库目录路径
    pub fn remotes_dir(&self) -> PathBuf {
        self.ledger_dir.join("remotes")
    }

    pub(crate) fn checked_remotes_dir(&self) -> Result<PathBuf> {
        let remotes_dir = self.remotes_dir();
        match remotes_dir.try_exists() {
            Ok(true) => {}
            Ok(false) => {
                anyhow::bail!(
                    "Broken remote repo catalog: remote repo directory missing at {:?}",
                    remotes_dir
                );
            }
            Err(err) => {
                return Err(err.into());
            }
        }
        Ok(remotes_dir)
    }

    pub(crate) fn checked_local_dir_for(ledger_dir: &Path, context: &str) -> Result<PathBuf> {
        let local_dir = ledger_dir.join("local");
        match local_dir.try_exists() {
            Ok(true) => Ok(local_dir),
            Ok(false) => Err(anyhow!(
                "Broken local repo catalog: local repo directory missing at {:?}",
                local_dir
            )),
            Err(err) => Err(err).with_context(|| {
                format!(
                    "Failed to stat local repo directory while {}: {:?}",
                    context, local_dir
                )
            }),
        }
    }

    /// 获取本地数据库的只读事务 (用于高级查询)
    pub fn local_db_read_txn(&self) -> Result<redb::ReadTransaction> {
        Ok(self.local_db.begin_read()?)
    }

    /// 获取本地库指定序列号范围的操作 (用于 P2P 同步增量推送)
    pub fn get_local_ops_in_range(
        &self,
        repo_id: &RepoId,
        start_seq: u64,
        end_seq: u64,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        self.run_on_local_repo_id(repo_id, |db| {
            range::get_ops_in_range(db, start_seq, end_seq)
        })
    }

    pub(crate) fn run_on_local_repo_stem<F, R>(&self, stem: &str, f: F) -> Result<R>
    where
        F: FnOnce(&redb::Database) -> Result<R>,
    {
        if stem == self.local_repo_name {
            source_control::validate_tables(self.local_db.as_ref()).map_err(|err| {
                anyhow!(
                    "Broken local repo {} while validating source control tables: {}",
                    self.local_repo_name,
                    err
                )
            })?;
            return f(self.local_db.as_ref());
        }
        {
            let guard = self.read_extra_local_dbs()?;
            if let Some(db) = guard.get(stem) {
                return f(db);
            }
        }
        let db_path = self.ledger_dir.join("local").join(format!("{}.redb", stem));
        let db = cached_database(&db_path)
            .map_err(|err| anyhow!("Broken local repo {} while opening database: {}", stem, err))?;
        source_control::validate_tables(db.as_ref()).map_err(|err| {
            anyhow!(
                "Broken local repo {} while validating source control tables: {}",
                stem,
                err
            )
        })?;
        {
            let mut guard = self.write_extra_local_dbs()?;
            if let std::collections::hash_map::Entry::Vacant(e) = guard.entry(stem.to_string()) {
                e.insert(db);
            }
        }
        let guard = self.read_extra_local_dbs()?;
        guard
            .get(stem)
            .map(|db| f(db.as_ref()))
            .transpose()?
            .ok_or_else(|| anyhow!("Failed into cache repo"))
    }

    pub(crate) fn resolve_local_repo_stem(&self, selector: &str) -> Result<Option<String>> {
        if selector == self.local_repo_name {
            return Ok(Some(self.local_repo_name.clone()));
        }
        let local_dir = Self::checked_local_dir_for(&self.ledger_dir, "resolving local selector")?;
        for (path, stem) in redb_repo_entries(&local_dir, "resolving local selector")? {
            if stem == selector {
                return Ok(Some(stem));
            }
            if stem == self.local_repo_name {
                continue;
            }
            if let Err(err) = Self::read_repo_info_from_path(&path) {
                return Err(anyhow!(
                    "Broken local repo {} while resolving selector {}: {}",
                    stem,
                    selector,
                    err
                ));
            }
        }
        Ok(None)
    }

    fn read_extra_local_dbs(&self) -> Result<RwLockReadGuard<'_, HashMap<String, Arc<Database>>>> {
        self.extra_local_dbs
            .read()
            .map_err(|_| anyhow!("Local repo registry lock poisoned"))
    }

    fn write_extra_local_dbs(
        &self,
    ) -> Result<RwLockWriteGuard<'_, HashMap<String, Arc<Database>>>> {
        self.extra_local_dbs
            .write()
            .map_err(|_| anyhow!("Local repo registry lock poisoned"))
    }
}

#[cfg(test)]
#[path = "core_test.rs"]
mod tests;
