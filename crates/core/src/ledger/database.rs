// crates/core/src/ledger/database.rs
//! # 数据库访问模块 (Database Access)
//!
//! 提供获取数据库引用的方法，供会话级锁定使用。
//!
//! **设计说明**:
//! Redb 的 `Database::create()` 会获取独占文件锁，不能在同一进程中多次打开同一文件。
//! 因此，我们使用一个缓存 (`opened_dbs`) 来存储已打开的数据库的 Arc 引用。
//! 主库 (`local_db`) 已经被 RepoManager 持有，我们通过路径匹配来避免重复打开。

use super::RepoManager;
use super::database_cache::{
    CachedDatabaseEntry, OPENED_DBS, current_file_stamp, reusable_cached_database,
};
pub(crate) use super::database_cache::{register_database, relocate_database_path};
use crate::models::PeerId;
use crate::models::RepoId;
use anyhow::{Context, Result};
use redb::Database;
use std::path::Path;
use std::sync::Arc;

pub(crate) fn cached_database(db_path: &Path) -> Result<Arc<Database>> {
    if !checked_exists(db_path, "database path")? {
        return Err(anyhow::anyhow!("Repository not found: {:?}", db_path));
    }
    cached_or_create_database(db_path)
}

pub(crate) fn cached_or_create_database(db_path: &Path) -> Result<Arc<Database>> {
    if let Some(db) = reusable_cached_database(db_path)? {
        return Ok(db);
    }
    OPENED_DBS
        .write()
        .map_err(|_| anyhow::anyhow!("Database cache lock poisoned while removing {:?}", db_path))?
        .remove(db_path);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Database::create(db_path)?;
    let arc_db = Arc::new(db);

    {
        let mut cache = OPENED_DBS.write().map_err(|_| {
            anyhow::anyhow!("Database cache lock poisoned while storing {:?}", db_path)
        })?;
        cache.insert(
            db_path.to_path_buf(),
            CachedDatabaseEntry {
                db: arc_db.clone(),
                stamp: current_file_stamp(db_path)?,
            },
        );
    }

    tracing::info!("Opened and cached database: {:?}", db_path);
    Ok(arc_db)
}

/// 数据库访问信息
///
/// 包含数据库引用及其访问模式
#[derive(Clone)]
pub struct DatabaseHandle {
    /// 数据库引用
    pub db: Arc<Database>,
    /// 是否为只读模式 (remotes/ 下的数据库)
    pub readonly: bool,
    /// 分支标识 (None = local, Some = remote)
    pub branch: Option<PeerId>,
    /// 仓库 UUID（若已解析）
    pub repo_id: Option<RepoId>,
    /// 仓库名称
    pub repo_name: String,
}

impl RepoManager {
    fn repair_secondary_local_runtime_tables(
        &self,
        repo_name: &str,
        db: &Arc<Database>,
    ) -> Result<()> {
        if repo_name == self.local_repo_name {
            return Ok(());
        }
        if super::runtime_tables::repair_missing_client_op_index(db.as_ref())? {
            tracing::warn!(
                "Rebuilt missing client_op_index while opening local repo runtime tables: {}",
                repo_name
            );
        }
        Ok(())
    }

    /// 打开并返回指定分支和仓库的数据库句柄
    ///
    /// **参数**:
    /// - `branch`: None 表示本地分支, Some(peer_id) 表示远端影子库
    /// - `repo_name`: 仓库名称 (不含 .redb 后缀)
    ///
    /// **返回**:
    /// 包含数据库引用和访问模式的 `DatabaseHandle`
    ///
    /// **线程安全**:
    /// 使用全局缓存确保同一数据库文件在进程内只打开一次。
    pub fn open_database(
        &self,
        branch: Option<&PeerId>,
        repo_name: &str,
    ) -> Result<DatabaseHandle> {
        let name = repo_name.trim_end_matches(".redb");

        match branch {
            // 本地分支 (可读写)
            None => {
                let stem = self.resolve_local_repo_name_for_execution(None, Some(name))?;
                let db = self.get_or_open_local_db(&stem)?;
                let repo_id = self
                    .get_repo_info_for(None, Some(&stem))?
                    .map(|info| info.uuid)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Broken local repo {} while opening database: repository metadata missing",
                            stem
                        )
                    })?;
                Ok(DatabaseHandle {
                    db,
                    readonly: false,
                    branch: None,
                    repo_id: Some(repo_id),
                    repo_name: stem,
                })
            }
            // 远端影子库 (只读)
            Some(peer_id) => {
                let resolved = self
                    .resolve_remote_repo_entry(peer_id, name)?
                    .ok_or_else(|| anyhow::anyhow!("Repository not found: {}", name))?;
                let repo_name = resolved.stem.clone();
                let repo_id = match &resolved.info {
                    Some(info) => info.uuid,
                    None => uuid::Uuid::parse_str(&resolved.stem).map_err(|_| {
                        anyhow::anyhow!(
                            "Broken remote repo {} for peer {} while opening database: repository metadata missing",
                            resolved.stem,
                            peer_id
                        )
                    })?,
                };
                let loaded = self
                    .read_shadow_dbs()?
                    .get(peer_id)
                    .and_then(|repos| repos.get(&repo_id))
                    .cloned();
                if let Some(db) = loaded {
                    return Ok(DatabaseHandle {
                        db,
                        readonly: true,
                        branch: Some(peer_id.clone()),
                        repo_id: Some(repo_id),
                        repo_name,
                    });
                }
                let db = self.get_or_open_db_at(&resolved.path)?;
                Ok(DatabaseHandle {
                    db,
                    readonly: true, // 远端分支始终只读
                    branch: Some(peer_id.clone()),
                    repo_id: Some(repo_id),
                    repo_name,
                })
            }
        }
    }

    /// 获取或打开本地数据库 (返回 Arc)
    pub(crate) fn get_or_open_local_db(&self, name: &str) -> Result<Arc<Database>> {
        let cache_key = self.ledger_dir.join("local").join(format!("{}.redb", name));

        // 1. 检查全局缓存
        if let Some(db) = reusable_cached_database(cache_key.as_path())? {
            self.repair_secondary_local_runtime_tables(name, &db)?;
            return Ok(db);
        }
        OPENED_DBS
            .write()
            .map_err(|_| {
                anyhow::anyhow!(
                    "Database cache lock poisoned while removing {:?}",
                    cache_key
                )
            })?
            .remove(cache_key.as_path());

        // 2. 检查是否是主库 (已经被 RepoManager 持有)
        // 主库的路径检查
        let main_db_path = self
            .ledger_dir
            .join("local")
            .join(format!("{}.redb", self.local_repo_name));

        if cache_key == main_db_path {
            return Ok(self.local_db.clone());
        }

        // 3. 检查文件是否存在
        if !checked_exists(&cache_key, "local database path")? {
            return Err(anyhow::anyhow!("Repository not found: {}", name));
        }

        // 4. 打开新数据库并缓存
        let db = Database::create(&cache_key)?;
        let arc_db = Arc::new(db);
        self.repair_secondary_local_runtime_tables(name, &arc_db)?;

        {
            let mut cache = OPENED_DBS.write().map_err(|_| {
                anyhow::anyhow!("Database cache lock poisoned while storing {:?}", cache_key)
            })?;
            cache.insert(
                cache_key.clone(),
                CachedDatabaseEntry {
                    db: arc_db.clone(),
                    stamp: current_file_stamp(&cache_key)?,
                },
            );
        }

        tracing::info!("Opened and cached database: {:?}", cache_key);
        Ok(arc_db)
    }

    /// 获取或打开影子数据库 (返回 Arc)
    fn get_or_open_db_at(&self, db_path: &Path) -> Result<Arc<Database>> {
        cached_database(db_path)
    }
}

fn checked_exists(path: &Path, context: &str) -> Result<bool> {
    path.try_exists()
        .with_context(|| format!("Failed to stat {}: {:?}", context, path))
}

#[cfg(test)]
#[path = "database_test.rs"]
mod tests;
