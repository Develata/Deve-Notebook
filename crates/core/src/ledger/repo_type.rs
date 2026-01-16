//! # 仓库类型模块 (Repository Type)
//!
//! 定义 P2P 架构中的仓库类型枚举。
//!
//! ## 架构背景
//!
//! 根据 Trinity Isolation 架构,系统维护两种类型的仓库:
//! - **Store B (Local Repo)**: 本地权威库,只有本地用户操作能写入
//! - **Store C (Shadow Repos)**: 远端影子库,存储远端节点的数据副本

use crate::models::PeerId;
use uuid::Uuid;

/// 仓库 ID (UUID)
pub type RepoId = Uuid;

/// 仓库类型枚举
///
/// 用于指定操作的目标仓库,实现 Trinity Isolation 中的数据隔离。
///
/// # 变体说明
///
/// - `Local`: 本地权威库 (Store B),存储在 `ledger/local/{repo_id}.redb`
/// - `Remote(PeerId, RepoId)`: 远端影子库 (Store C),存储在 `ledger/remotes/{peer_id}/{repo_id}.redb`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RepoType {
    /// 本地权威库 (Store B)
    ///
    /// 物理路径: `{ledger_dir}/local/{repo_id}.redb`
    Local(RepoId),

    /// 远端影子库 (Store C)
    ///
    /// 物理路径: `{ledger_dir}/remotes/{peer_id}/{repo_id}.redb`
    Remote(PeerId, RepoId),
}

impl RepoType {
    /// 获取 RepoId
    pub fn repo_id(&self) -> RepoId {
        match self {
            RepoType::Local(id) => *id,
            RepoType::Remote(_, id) => *id,
        }
    }
}
