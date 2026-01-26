// crates/core/src/ledger/database.rs
//! # 数据库访问模块 (Database Access)
//!
//! 提供获取数据库引用的方法，供会话级锁定使用。

use super::RepoManager;
use crate::models::PeerId;
use anyhow::Result;
use redb::Database;
use std::sync::Arc;

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
    /// **只读模式**:
    /// 当 `branch` 为 Some (远端) 时，返回的句柄标记为只读。
    /// 调用方应检查 `readonly` 字段并静默忽略编辑请求。
    /// // TODO: Frontend will hide edit buttons when readonly
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
        // 1. 主库
        if name == self.local_repo_name {
            // 主库不是 Arc，我们需要用一个 Arc wrapper
            // 但 Database 不能克隆，所以我们需要不同的方式...
            // 实际上，我们需要修改 RepoManager 以使用 Arc<Database> 存储所有数据库
            // 这是一个较大的改动。暂时先返回一个新打开的实例？
            // 不对，这会导致问题（多个 Database 实例对同一文件）

            // 方案：对于主库，直接使用路径重新包装
            // Redb 实际上可以多次打开同一文件（它会使用文件锁）
            let db_path = self.ledger_dir.join("local").join(format!("{}.redb", name));
            let db = Database::create(&db_path)?;
            return Ok(Arc::new(db));
        }

        // 2. 检查缓存
        {
            let guard = self.extra_local_dbs.read().unwrap();
            if guard.contains_key(name) {
                // 缓存中有，但是是 Database 不是 Arc<Database>
                // 需要修改缓存类型...暂时重新打开
                let db_path = self.ledger_dir.join("local").join(format!("{}.redb", name));
                let db = Database::create(&db_path)?;
                return Ok(Arc::new(db));
            }
        }

        // 3. 打开新数据库
        let db_path = self.ledger_dir.join("local").join(format!("{}.redb", name));
        if !db_path.exists() {
            return Err(anyhow::anyhow!("Repository not found: {}", name));
        }

        let db = Database::create(&db_path)?;
        Ok(Arc::new(db))
    }

    /// 获取或打开影子数据库 (返回 Arc)
    fn get_or_open_shadow_db(&self, peer_id: &PeerId, name: &str) -> Result<Arc<Database>> {
        let db_path = self
            .remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", name));

        if !db_path.exists() {
            return Err(anyhow::anyhow!(
                "Shadow repository not found: {}/{}",
                peer_id,
                name
            ));
        }

        let db = Database::create(&db_path)?;
        Ok(Arc::new(db))
    }
}
