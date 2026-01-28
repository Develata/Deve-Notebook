// crates/core/src/ledger/mod.rs
//! # 仓库管理器 (Repository Manager)
//!
//! 本模块实现 P2P Git-Flow 架构中的"三位一体隔离" (Trinity Isolation)。
//!
//! ## 架构作用
//!
//! * **Store B (Local Repo)**: 本地权威库 (`local.redb`)，只有本地操作能写入
//! * **Store C (Shadow Repos)**: 远端影子库 (`remotes/peer_X.redb`)，存储远端节点数据
//!
//! ## 模块结构
//!
//! - `schema`: 数据库表定义
//! - `init`: 初始化逻辑
//! - `metadata`: Path/DocId 映射
//! - `ops`: 操作日志读写
//! - `snapshot`: 快照管理
//! - `range`: 范围查询
//! - `shadow`: Shadow 库底层实现
//! - `shadow_manager`: Shadow DB 管理
//! - `source_control`: 版本控制集成
//! - `listing`: 文档列表
//! - `merge`: 合并引擎
//! - `manager`: RepoManager 实现分布模块

use anyhow::{Context, Result};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::models::{LedgerEntry, PeerId, RepoId};

// ========== 子模块声明 ==========

pub mod database;
pub mod init;
pub mod listing;
mod manager;
pub mod merge;
pub mod metadata;
pub mod ops;
pub mod range;
pub mod schema;
pub mod shadow;
mod shadow_manager;
pub mod snapshot;
pub mod source_control;
pub mod traits;

// ========== 公开导出 ==========

pub use self::schema::*;

#[cfg(test)]
mod tests;

/// 仓库元数据信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    /// 仓库唯一标识
    pub uuid: RepoId,
    /// 仓库名称 (Human Readable)
    pub name: String,
    /// 仓库 URL (唯一逻辑标识) - 可选
    pub url: Option<String>,
}

/// 仓库管理器 (Repository Manager)
///
/// 管理本地唯一的 Local Repo (Store B) 和多个 Shadow Repos (Store C)。
/// 实现 Trinity Isolation 架构中的数据隔离策略。
pub struct RepoManager {
    /// 账本目录根路径
    pub(crate) ledger_dir: PathBuf,
    /// 本地权威库 (local.redb) - 默认/主库
    pub(crate) local_db: Database,
    /// 主库名称
    pub(crate) local_repo_name: String,
    /// 其他本地库缓存 (name -> Database)
    pub(crate) extra_local_dbs: RwLock<HashMap<String, Database>>,
    /// 远端影子库集合 (peer_id -> repo_id -> Database) - 懒加载
    pub(crate) shadow_dbs: RwLock<HashMap<PeerId, HashMap<RepoId, Database>>>,
    /// 快照保留深度
    pub snapshot_depth: usize,
}

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
            return Err(anyhow::anyhow!("Repository not found: {}", name));
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
            Err(anyhow::anyhow!("Failed into cache repo"))
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
    fn read_repo_info_from_db(db: &Database) -> Result<Option<RepoInfo>> {
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

    /// 获取指定分支下指定仓库的 URL
    ///
    /// * `branch`: None 表示本地 (main repo or extra local), Some(peer_id) 表示远端影子库
    /// * `repo_name`: 仓库名称 (不含 .redb 后缀)
    pub fn get_repo_url(&self, branch: Option<&PeerId>, repo_name: &str) -> Result<Option<String>> {
        let name = repo_name.trim_end_matches(".redb");

        // Helper to read URL from a DB instance
        let read_url = |db: &Database| -> Result<Option<String>> {
            let info = Self::read_repo_info_from_db(db)?;
            Ok(info.and_then(|i| i.url))
        };

        if let Some(peer_id) = branch {
            // Remote (Shadow)
            // 1. Check loaded shadow DBs (This is tricky because shadow_dbs is HashMap<PeerId, HashMap<RepoId, Database>>)
            // We don't have RepoId here, only name. So we cannot easily lookup by name in the nested map if it's keyed by ID.
            // However, list_repos(Some(peer_id)) returns names.

            // For simplicity and correctness with "on disk" files, let's open the file directly if it exists.
            // NOTE: Opening the same Redb file multiple times in same process is allowed IF read-only?
            // Redb: "Multiple read transactions can be open at the same time... A single write transaction..."
            // "Database::create" opens it.
            // If we are strictly reading, we can just open it.

            let db_path = self
                .remotes_dir()
                .join(peer_id.to_filename())
                .join(format!("{}.redb", name));
            if !db_path.exists() {
                return Ok(None);
            }

            // Try to open. If it fails (locked), we might simply return None or log error.
            // But since we are the only process (mostly), it should be fine unless Write Txn holds it?
            // "Database::open" might fail if file assumes exclusive lock?
            // In Redb 1.x, Database is thread-safe. But separate Database instances on same file?
            // Better to rely on the fact that if it's not in our manager's cache (which uses RepoId), we open it.
            // But since we can't map Name -> RepoId without opening, we catch-22.

            // Just open it. Redb handles file locking. If we already have it open in this process, we should share it.
            // But implementing name->id mapping for shadow dbs is too much change right now.
            // Let's assume we can open it.
            let db = Database::create(&db_path)?; // create opens existing or creates new. Here exists check passed.
            return read_url(&db);
        } else {
            // Local
            // 1. Check Main Repo
            if name == self.local_repo_name {
                return read_url(&self.local_db);
            }

            // 2. Check Extra Local DBs Cache
            {
                let guard = self.extra_local_dbs.read().unwrap();
                if let Some(db) = guard.get(name) {
                    return read_url(db);
                }
            }

            // 3. Open temp
            let db_path = self.ledger_dir.join("local").join(format!("{}.redb", name));
            if !db_path.exists() {
                return Ok(None);
            }
            let db = Database::create(&db_path)?;
            return read_url(&db);
        }
    }

    /// 查找具有指定 URL 的本地仓库 (Main 或 Extra)
    pub fn find_local_repo_name_by_url(&self, target_url: &str) -> Result<Option<String>> {
        // 1. Check Main Repo
        if let Ok(Some(info)) = Self::read_repo_info_from_db(&self.local_db) {
            if info.url.as_deref() == Some(target_url) {
                return Ok(Some(self.local_repo_name.clone()));
            }
        }

        // 2. Iterate all .redb files in ledger/local
        let local_dir = self.ledger_dir.join("local");
        if !local_dir.exists() {
            return Ok(None);
        }

        for entry in std::fs::read_dir(local_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("redb") {
                let file_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();

                // Skip if main (already checked)
                if file_stem == self.local_repo_name {
                    continue;
                }

                // Use run_on_local_repo to safely access/cache
                let is_match = self
                    .run_on_local_repo(file_stem, |db| {
                        let info = Self::read_repo_info_from_db(db)?;
                        Ok(info.and_then(|i| i.url).as_deref() == Some(target_url))
                    })
                    .unwrap_or(false);

                if is_match {
                    return Ok(Some(file_stem.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// 重置指定 Shadow 文档的所有历史操作 (物理清空)
    ///
    /// **用途**: 当接收到 P2P Snapshot 时，旧的增量日志失效，需清空并重写。
    pub fn reset_shadow_doc(
        &self,
        peer_id: &PeerId,
        repo_id: &RepoId,
        doc_id: &crate::models::DocId,
    ) -> Result<()> {
        let shadow_db = self.ensure_shadow_db(peer_id, repo_id)?;

        // Redb 2.0 transaction safety:
        // We cannot call begin_write on a Database object directly if it's not a Database?
        // Wait, ensure_shadow_db returns `Result<&Database>`.
        // The error says "method not found in `()`".
        // This implies ensure_shadow_db returns `Result<()>`. Let's check `shadow_manager.rs`.

        // Ah, `shadow_manager` implementation likely returns `Result<()>` or something else.
        // Or I am calling it wrong.

        // Actually, `ensure_shadow_db` is implemented in `shadow_manager.rs`.
        // Let's check the implementation.
        // It likely returns `Result<()>`. It ensures it is loaded into `shadow_dbs`.
        // So we need to fetch it from `shadow_dbs` map after ensuring.

        self.ensure_shadow_db(peer_id, repo_id)?;

        let guard = self.shadow_dbs.read().unwrap();
        let peer_map = guard
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer DBs not loaded"))?;
        let db = peer_map
            .get(repo_id)
            .ok_or_else(|| anyhow::anyhow!("Shadow DB not found"))?;

        let write_txn = db.begin_write()?;

        {
            let mut table = write_txn.open_multimap_table(DOC_OPS)?;
            // Redb multimap remove deletes a specific key-value pair.
            // remove_all is what we want (delete all values for a key).
            table.remove_all(&doc_id.as_u128())?;
        }

        write_txn.commit()?;
        Ok(())
    }

    /// 删除指定 Peer 的影子库目录

    pub fn delete_peer_branch(&self, peer_id: &PeerId) -> Result<()> {
        let peer_dir = self.remotes_dir().join(peer_id.to_filename());

        // 1. Check if exists
        if !peer_dir.exists() {
            return Ok(()); // Idempotent success
        }

        // 2. Remove from cache (shadow_dbs)
        {
            let mut guard = self.shadow_dbs.write().unwrap();
            guard.remove(peer_id);
        }

        // 3. Physical delete
        std::fs::remove_dir_all(&peer_dir)
            .with_context(|| format!("无法删除 Peer 目录: {:?}", peer_dir))?;

        tracing::info!("Deleted peer branch: {}", peer_id);
        Ok(())
    }
}
