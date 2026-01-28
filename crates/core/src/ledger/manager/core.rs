use anyhow::{anyhow, Result};
use redb::Database;
use std::path::{Path, PathBuf};

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::ledger::schema::*;
use crate::ledger::{init, metadata, range};
use crate::models::{LedgerEntry, RepoId};

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

    /// 执行闭包于指定的本地仓库 (按名称)
    ///
    /// * `repo_name`: 仓库名称 (e.g. "default", "wiki").
    /// * `f`: 接收 &Database 的闭包.
    pub fn run_on_local_repo<F, R>(&self, repo_name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&Database) -> Result<R>,
    {
        // 1. Check Main Repo, strip extension if present
        let name = repo_name.trim_end_matches(".redb");

        if name == self.local_repo_name {
            return f(&self.local_db);
        }

        // 2. Check Cache
        {
            let guard = self.extra_local_dbs.read().unwrap();
            if let Some(db) = guard.get(name) {
                return f(db);
            }
        }

        // 3. Open if not cached
        let db_path = self.ledger_dir.join("local").join(format!("{}.redb", name));
        if !db_path.exists() {
            return Err(anyhow!("Repository not found: {}", name));
        }

        let db = Database::create(&db_path)?;

        // Cache it
        {
            let mut guard = self.extra_local_dbs.write().unwrap();
            if let std::collections::hash_map::Entry::Vacant(e) = guard.entry(name.to_string()) {
                e.insert(db);
            }
        }

        // 4. Run closure
        let guard = self.extra_local_dbs.read().unwrap();
        if let Some(db) = guard.get(name) {
            f(db)
        } else {
            Err(anyhow!("Failed into cache repo"))
        }
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
        self.run_on_local_repo(name, |db| metadata::list_docs(db))
    }

    /// 获取账本目录路径
    pub fn ledger_dir(&self) -> &Path {
        &self.ledger_dir
    }

    /// 获取影子库目录路径
    pub fn remotes_dir(&self) -> PathBuf {
        self.ledger_dir.join("remotes")
    }

    /// 获取本地数据库的只读事务 (用于高级查询)
    pub fn local_db_read_txn(&self) -> Result<redb::ReadTransaction> {
        Ok(self.local_db.begin_read()?)
    }

    /// 获取本地库指定序列号范围的操作 (用于 P2P 同步增量推送)
    pub fn get_local_ops_in_range(
        &self,
        _repo_id: &RepoId,
        start_seq: u64,
        end_seq: u64,
    ) -> Result<Vec<(u64, LedgerEntry)>> {
        range::get_ops_in_range(&self.local_db, start_seq, end_seq)
    }

    /// 获取本地仓库的元数据信息 (UUID, Name, URL)
    pub fn get_repo_info(&self) -> Result<Option<RepoInfo>> {
        Self::read_repo_info_from_db(&self.local_db)
    }

    /// 从指定数据库读取 RepoInfo
    pub(crate) fn read_repo_info_from_db(db: &Database) -> Result<Option<RepoInfo>> {
        let read_txn = db.begin_read()?;
        // Try open table, if not exists return None
        let table = match read_txn.open_table(REPO_METADATA) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        if let Some(guard) = table.get(&0)? {
            let value = guard.value();
            let info: RepoInfo = bincode::deserialize(value)?;
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
}
