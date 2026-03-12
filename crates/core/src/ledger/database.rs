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
use crate::models::PeerId;
use crate::models::RepoId;
use anyhow::Result;
use redb::Database;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// 全局缓存：已打开的数据库 (path -> Arc<Database>)
/// 这确保同一个数据库文件在整个进程中只被打开一次
static OPENED_DBS: std::sync::LazyLock<RwLock<HashMap<std::path::PathBuf, Arc<Database>>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

pub(crate) fn register_database(db_path: &Path, db: Arc<Database>) {
    OPENED_DBS
        .write()
        .unwrap()
        .insert(db_path.to_path_buf(), db);
}

pub(crate) fn relocate_database_path(old_path: &Path, new_path: &Path) {
    let mut cache = OPENED_DBS.write().unwrap();
    if let Some(db) = cache.remove(old_path) {
        cache.insert(new_path.to_path_buf(), db);
    }
}

pub(crate) fn cached_database(db_path: &Path) -> Result<Arc<Database>> {
    if !db_path.exists() {
        return Err(anyhow::anyhow!("Repository not found: {:?}", db_path));
    }
    cached_or_create_database(db_path)
}

pub(crate) fn cached_or_create_database(db_path: &Path) -> Result<Arc<Database>> {
    {
        let cache = OPENED_DBS.read().unwrap();
        if let Some(arc_db) = cache.get(db_path) {
            tracing::debug!("Database cache hit: {:?}", db_path);
            return Ok(arc_db.clone());
        }
    }
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Database::create(db_path)?;
    let arc_db = Arc::new(db);

    {
        let mut cache = OPENED_DBS.write().unwrap();
        cache.insert(db_path.to_path_buf(), arc_db.clone());
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
                let db = self.get_or_open_local_db(name)?;
                let repo_id = self
                    .get_repo_info_for(None, Some(name))?
                    .map(|info| info.uuid);
                Ok(DatabaseHandle {
                    db,
                    readonly: false,
                    branch: None,
                    repo_id,
                    repo_name: name.to_string(),
                })
            }
            // 远端影子库 (只读)
            Some(peer_id) => {
                let resolved = self.resolve_remote_repo_entry(peer_id, name)?;
                let repo_name = resolved
                    .as_ref()
                    .and_then(|entry| entry.info.as_ref().map(|info| info.name.clone()))
                    .or_else(|| {
                        resolved.as_ref().and_then(|entry| {
                            uuid::Uuid::parse_str(&entry.stem)
                                .ok()
                                .and_then(|repo_id| {
                                    self.get_local_repo_info_by_id(repo_id).ok().flatten()
                                })
                                .map(|info| info.name)
                        })
                    })
                    .unwrap_or_else(|| name.to_string());
                let repo_id = resolved.as_ref().and_then(|entry| {
                    entry
                        .info
                        .as_ref()
                        .map(|info| info.uuid)
                        .or_else(|| uuid::Uuid::parse_str(&entry.stem).ok())
                });
                let loaded = repo_id.and_then(|repo_id| {
                    self.shadow_dbs
                        .read()
                        .unwrap()
                        .get(peer_id)
                        .and_then(|repos| repos.get(&repo_id))
                        .cloned()
                });
                if let Some(db) = loaded {
                    return Ok(DatabaseHandle {
                        db,
                        readonly: true,
                        branch: Some(peer_id.clone()),
                        repo_id,
                        repo_name,
                    });
                }
                let db_path = resolved.map(|entry| entry.path).unwrap_or_else(|| {
                    self.remotes_dir()
                        .join(peer_id.to_filename())
                        .join(format!("{}.redb", name))
                });
                let db = self.get_or_open_db_at(&db_path)?;
                Ok(DatabaseHandle {
                    db,
                    readonly: true, // 远端分支始终只读
                    branch: Some(peer_id.clone()),
                    repo_id,
                    repo_name,
                })
            }
        }
    }

    /// 获取或打开本地数据库 (返回 Arc)
    fn get_or_open_local_db(&self, name: &str) -> Result<Arc<Database>> {
        let cache_key = self.ledger_dir.join("local").join(format!("{}.redb", name));

        // 1. 检查全局缓存
        {
            let cache = OPENED_DBS.read().unwrap();
            if let Some(arc_db) = cache.get(cache_key.as_path()) {
                tracing::debug!("Database cache hit: {:?}", cache_key);
                return Ok(arc_db.clone());
            }
        }

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
        if !cache_key.exists() {
            return Err(anyhow::anyhow!("Repository not found: {}", name));
        }

        // 4. 打开新数据库并缓存
        let db = Database::create(&cache_key)?;
        let arc_db = Arc::new(db);

        {
            let mut cache = OPENED_DBS.write().unwrap();
            cache.insert(cache_key.clone(), arc_db.clone());
        }

        tracing::info!("Opened and cached database: {:?}", cache_key);
        Ok(arc_db)
    }

    /// 获取或打开影子数据库 (返回 Arc)
    fn get_or_open_db_at(&self, db_path: &Path) -> Result<Arc<Database>> {
        cached_database(db_path)
    }
}
