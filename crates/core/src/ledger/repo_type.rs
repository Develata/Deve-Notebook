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

/// 仓库类型枚举
///
/// 用于指定操作的目标仓库,实现 Trinity Isolation 中的数据隔离。
///
/// # 变体说明
///
/// - `Local`: 本地权威库 (Store B),存储在 `local.redb`
/// - `Remote(PeerId)`: 远端影子库 (Store C),存储在 `remotes/{peer_id}.redb`
///
/// # 示例
///
/// ```ignore
/// use deve_core::ledger::RepoType;
/// use deve_core::models::PeerId;
///
/// // 指定本地库
/// let local = RepoType::Local;
///
/// // 指定某个远端 Peer 的影子库
/// let remote = RepoType::Remote(PeerId::new("peer_mobile"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoType {
    /// 本地权威库 (Store B)
    ///
    /// 物理路径: `{ledger_dir}/local.redb`
    Local,

    /// 远端影子库 (Store C)
    ///
    /// 物理路径: `{ledger_dir}/remotes/{peer_id}.redb`
    Remote(PeerId),
}
