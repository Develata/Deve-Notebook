// apps/cli/src/server/session.rs
//! # WebSocket 会话状态 (Session State)
//!
//! **功能**:
//! 管理单个 WebSocket 连接的会话状态。
//!
//! **状态内容**:
//! - `authenticated_peer_id`: P2P 握手后的对端 ID
//! - `active_branch`: 当前活动分支 (None = 本地, Some = 影子库)

use deve_core::models::PeerId;

/// WebSocket 会话状态
///
/// 每个 WebSocket 连接维护独立的会话状态实例。
#[derive(Debug, Default)]
pub struct WsSession {
    /// 已认证的对端 Peer ID
    ///
    /// 在 SyncHello 握手成功后设置，用于后续 SyncPush 验证。
    pub authenticated_peer_id: Option<PeerId>,

    /// 当前活动分支
    ///
    /// - `None`: 本地分支 (Master)
    /// - `Some(peer_id)`: 远程影子库 (只读模式)
    pub active_branch: Option<PeerId>,
}

impl WsSession {
    /// 创建新会话
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置已认证的 Peer ID
    pub fn set_authenticated(&mut self, peer_id: PeerId) {
        self.authenticated_peer_id = Some(peer_id);
    }

    /// 切换活动分支
    ///
    /// 传入 `None` 切换回本地分支，传入 `Some(id)` 切换到影子库。
    pub fn switch_branch(&mut self, peer_id: Option<String>) {
        self.active_branch = peer_id.map(|id| PeerId::new(id));
    }

    /// 检查是否在影子分支 (只读模式)
    pub fn is_readonly(&self) -> bool {
        self.active_branch.is_some()
    }
}
