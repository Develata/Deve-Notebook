//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 06_repository#repo-scope-runtime
//!   - 04_storage#repo-runtime-layout
//!
use crate::models::{PeerId, RepoId};
use crate::sync::persist_guard::PersistGuard;
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// 仓库元数据信息
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub(crate) local_db: Arc<Database>,
    /// 主库名称
    pub(crate) local_repo_name: String,
    /// 其他本地库缓存 (name -> Database)。不变量：同一路径在进程内只打开一次。
    pub(crate) extra_local_dbs: RwLock<HashMap<String, Arc<Database>>>,
    /// 远端影子库集合 (peer_id -> repo_id -> Database) - 懒加载
    pub(crate) shadow_dbs: RwLock<HashMap<PeerId, HashMap<RepoId, Arc<Database>>>>,
    /// 快照保留深度
    pub snapshot_depth: usize,
    /// Vault 根目录 (用于 commit 时读取磁盘文件内容)
    pub(crate) vault_root: Option<PathBuf>,
    /// 受控 Projection 写回的 watcher 忽略表。
    pub(crate) persist_guard: Arc<PersistGuard>,
}
