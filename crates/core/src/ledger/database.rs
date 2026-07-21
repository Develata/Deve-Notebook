// crates/core/src/ledger/database.rs
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#repo-catalog-contract
//!
//! # 数据库访问模块 (Database Access)
//!
//! 提供 repo scope binding 与远端只读数据库引用。
//!
//! **设计说明**:
//! 本地 Redb 只由 `LocalAuthorityRuntime` 持有，session handle 仅保存 RepoId binding。
//! 远端 shadow DB 仍可使用 Arc，因为它不属于本地 authority retirement 边界。

use super::RepoManager;
pub(crate) use super::database_cache::relocate_database_path;
pub(crate) use super::database_open::{cached_or_create_shadow_database, cached_shadow_database};
use crate::models::PeerId;
use crate::models::RepoId;
use anyhow::Result;
use redb::Database;
use std::path::Path;
use std::sync::Arc;

mod runtime;

/// 数据库访问信息
///
/// 包含数据库引用及其访问模式
#[derive(Clone)]
pub struct DatabaseHandle {
    remote_db: Option<Arc<Database>>,
    /// 是否为只读模式 (remotes/ 下的数据库)
    pub readonly: bool,
    /// 分支标识 (None = local, Some = remote)
    pub branch: Option<PeerId>,
    /// 仓库 UUID（若已解析）
    pub repo_id: Option<RepoId>,
    /// 仓库名称
    pub repo_name: String,
}

impl DatabaseHandle {
    pub fn local(repo_id: RepoId, repo_name: String) -> Self {
        Self {
            remote_db: None,
            readonly: false,
            branch: None,
            repo_id: Some(repo_id),
            repo_name,
        }
    }

    pub fn remote(db: Arc<Database>, peer_id: PeerId, repo_id: RepoId, repo_name: String) -> Self {
        Self {
            remote_db: Some(db),
            readonly: true,
            branch: Some(peer_id),
            repo_id: Some(repo_id),
            repo_name,
        }
    }

    /// Creates a read-only remote binding whose RepoId has not been resolved.
    /// Such a binding can never satisfy a RepoId-scoped authority check.
    pub fn unresolved_remote(db: Arc<Database>, peer_id: PeerId, repo_name: String) -> Self {
        Self {
            remote_db: Some(db),
            readonly: true,
            branch: Some(peer_id),
            repo_id: None,
            repo_name,
        }
    }

    pub fn remote_db(&self) -> Result<&Database> {
        self.remote_db
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("local session binding does not own a database handle"))
    }
}

impl RepoManager {
    pub(crate) fn database_runtime(&self) -> runtime::RepoDatabaseRuntime<'_> {
        runtime::RepoDatabaseRuntime::new(self)
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
        self.database_runtime().open_database(branch, repo_name)
    }

    /// 获取或打开影子数据库 (返回 Arc)
    fn get_or_open_db_at(&self, db_path: &Path) -> Result<Arc<Database>> {
        cached_shadow_database(db_path)
    }
}

#[cfg(test)]
mod tests;
