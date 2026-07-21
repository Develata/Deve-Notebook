//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/index#repo-runtime-layout
//!
use super::authority_storage_runtime::{LocalAuthorityRuntime, PreparedRepoAuthority};
use super::repo_catalog_runtime::CatalogMembershipRuntime;
use crate::models::{PeerId, RepoId};
use crate::writeback::PersistGuard;
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalRepoSummary {
    pub repo_id: RepoId,
    pub name: String,
    pub execution_name: String,
}

/// 仓库管理器 (Repository Manager)
///
/// 管理本地唯一的 Local Repo (Store B) 和多个 Shadow Repos (Store C)。
/// 实现 Trinity Isolation 架构中的数据隔离策略。
pub struct RepoManager {
    /// 账本目录根路径
    pub(crate) ledger_dir: PathBuf,
    /// Physical host identity used for every locally-authored fact.
    pub(crate) local_peer_id: PeerId,
    /// 唯一拥有全部本地 Redb handle、generation 与跨进程锁的 runtime。
    pub(crate) local_authority: LocalAuthorityRuntime,
    /// Unique capability for the first local repo before its durable Normal
    /// catalog cut. Ordinary local-authority access remains membership-gated.
    pub(crate) initial_prepared_authority: Mutex<Option<PreparedRepoAuthority>>,
    /// 远端影子库集合 (peer_id -> repo_id -> Database) - 懒加载
    pub(crate) shadow_dbs: RwLock<HashMap<PeerId, HashMap<RepoId, Arc<Database>>>>,
    /// Serializes authenticated shadow mutation with merge checkpoint evaluation/commit.
    pub(crate) shadow_merge_guard: Mutex<()>,
    /// 快照保留深度
    pub snapshot_depth: usize,
    /// 受控 Projection 写回的 watcher 忽略表。
    pub(crate) persist_guard: Arc<PersistGuard>,
    /// Process-local catalog membership readiness authority.
    pub(crate) catalog_membership: CatalogMembershipRuntime,
}
