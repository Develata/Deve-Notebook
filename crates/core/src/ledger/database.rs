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
use anyhow::Result;
use redb::Database;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 全局缓存：已打开的数据库 (path -> Arc<Database>)
/// 这确保同一个数据库文件在整个进程中只被打开一次
static OPENED_DBS: std::sync::LazyLock<RwLock<HashMap<std::path::PathBuf, Arc<Database>>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

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
                Ok(DatabaseHandle {
                    db,
                    readonly: false,
                    branch: None,
                    repo_name: name.to_string(),
                })
            }
            // 远端影子库 (只读)
            Some(peer_id) => {
                let db = self.get_or_open_shadow_db(peer_id, name)?;
                Ok(DatabaseHandle {
                    db,
                    readonly: true, // 远端分支始终只读
                    branch: Some(peer_id.clone()),
                    repo_name: name.to_string(),
                })
            }
        }
    }

    /// 获取或打开本地数据库 (返回 Arc)
    fn get_or_open_local_db(&self, name: &str) -> Result<Arc<Database>> {
        let db_path = self.ledger_dir.join("local").join(format!("{}.redb", name));

        // 1. 检查全局缓存
        {
            let cache = OPENED_DBS.read().unwrap();
            if let Some(arc_db) = cache.get(&db_path) {
                tracing::debug!("Database cache hit: {:?}", db_path);
                return Ok(arc_db.clone());
            }
        }

        // 2. 检查是否是主库 (已经被 RepoManager 持有)
        // 主库的路径检查
        let main_db_path = self
            .ledger_dir
            .join("local")
            .join(format!("{}.redb", self.local_repo_name));

        if db_path == main_db_path {
            // 主库已经被 self.local_db 持有
            // 我们需要返回一个指向它的 Arc，但 self.local_db 不是 Arc
            // 解决方案：使用 run_on_local_repo 闭包模式而不是直接返回引用
            // 但现在的 API 期望 Arc<Database>...
            //
            // 临时方案：跳过主库的锁定检查，直接使用
            // 这意味着对于主库，我们不存入缓存，每次通过 run_on_local_repo 访问
            // 但这会让 open_database 对主库失败
            //
            // 更好的方案：通知调用者主库已可用，不需要重新打开
            return Err(anyhow::anyhow!(
                "Main local database is already managed by RepoManager. Use run_on_local_repo() for operations on '{}'",
                name
            ));
        }

        // 3. 检查文件是否存在
        if !db_path.exists() {
            return Err(anyhow::anyhow!("Repository not found: {}", name));
        }

        // 4. 打开新数据库并缓存
        let db = Database::create(&db_path)?;
        let arc_db = Arc::new(db);

        {
            let mut cache = OPENED_DBS.write().unwrap();
            cache.insert(db_path.clone(), arc_db.clone());
        }

        tracing::info!("Opened and cached database: {:?}", db_path);
        Ok(arc_db)
    }

    /// 获取或打开影子数据库 (返回 Arc)
    fn get_or_open_shadow_db(&self, peer_id: &PeerId, name: &str) -> Result<Arc<Database>> {
        let db_path = self
            .remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", name));

        // 1. 检查全局缓存
        {
            let cache = OPENED_DBS.read().unwrap();
            if let Some(arc_db) = cache.get(&db_path) {
                tracing::debug!("Shadow database cache hit: {:?}", db_path);
                return Ok(arc_db.clone());
            }
        }

        // 2. 检查文件是否存在
        if !db_path.exists() {
            return Err(anyhow::anyhow!(
                "Shadow repository not found: {}/{}",
                peer_id,
                name
            ));
        }

        // 3. 打开新数据库并缓存
        let db = Database::create(&db_path)?;
        let arc_db = Arc::new(db);

        {
            let mut cache = OPENED_DBS.write().unwrap();
            cache.insert(db_path.clone(), arc_db.clone());
        }

        tracing::info!("Opened and cached shadow database: {:?}", db_path);
        Ok(arc_db)
    }
}
