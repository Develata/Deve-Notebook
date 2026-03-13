// apps/cli/src/server/session.rs
//! # WebSocket 会话状态 (Session State)
//!
//! **功能**:
//! 管理单个 WebSocket 连接的会话状态。
//!
//! **状态内容**:
//! - `authenticated_peer_id`: P2P 握手后的对端 ID
//! - `writer_identity`: 浏览器写入身份（repo-scoped）
//! - `active_branch`: 当前活动分支 (None = 本地, Some = 影子库)
//! - `active_db`: 当前锁定的数据库句柄

use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use deve_core::models::RepoId;
use std::time::{Duration, Instant};

const WS_MESSAGE_WINDOW: Duration = Duration::from_secs(60);
const WS_MAX_MESSAGES_PER_WINDOW: u16 = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriterIdentity {
    pub peer_id: PeerId,
    pub repo_id: RepoId,
}

/// WebSocket 会话状态
///
/// 每个 WebSocket 连接维护独立的会话状态实例。
#[allow(dead_code)] // 为 P2P 握手和分支切换预留的字段
pub struct WsSession {
    /// 当前 UI/控制面 scope 的代际 token。
    ///
    /// Invariant:
    /// - 同一连接每次 branch/repo 切换都必须推进到新的 scope nonce。
    /// - 所有 repo-scoped 系统消息都必须携带当前 nonce，供前端丢弃跨代迟到消息。
    pub current_scope_nonce: u64,

    /// 当前同步握手代际 token。
    ///
    /// Invariant:
    /// - 每次 `SyncHello` 成功后都必须更新为客户端声明的 `scope_nonce`。
    /// - `SyncPush/SyncPushSnapshot` 必须沿用该 nonce，避免同 repo 重连时迟到增量串入新握手代。
    pub current_sync_scope_nonce: Option<u64>,

    /// 该连接是否来自已登录浏览器会话。
    ///
    /// Invariant:
    /// - JWT 认证通过的 Web Thin Client 置为 `true`。
    /// - 浏览器 SyncHello 仅用于 repo-scoped thin-client 协商，不应被当作 shadow branch 物化。
    pub browser_session: bool,

    /// 已认证的对端 Peer ID
    ///
    /// 在 SyncHello 握手成功后设置，用于后续 SyncPush 验证。
    pub authenticated_peer_id: Option<PeerId>,

    /// 当前绑定的仓库 ID (在 SyncHello 成功后设置)
    ///
    /// 用于后续同步消息的 repo 一致性校验。
    pub bound_repo_id: Option<RepoId>,

    /// 浏览器写入身份。
    ///
    /// Invariant:
    /// - 仅在当前连接已完成 repo-scoped sync handshake 后才可注册。
    /// - repo 切换或 peer 变更后必须失效。
    pub writer_identity: Option<WriterIdentity>,

    /// 当前活动分支。`None` 为本地分支，`Some(peer_id)` 为远程影子库。
    pub active_branch: Option<PeerId>,

    /// 当前活动仓库名称。`None` 表示默认仓库，`Some(name)` 表示指定 `.redb`。
    pub active_repo: Option<String>,

    /// 当前活动仓库 ID（UUID-first）
    pub active_repo_id: Option<RepoId>,

    /// 当前锁定的数据库句柄
    ///
    /// 在切换 branch/repo 时更新，所有后续操作使用此句柄
    pub active_db: Option<DatabaseHandle>,

    /// WebSocket 固定时间窗限流状态。
    ///
    /// Invariant:
    /// - 每个连接独立计数。
    /// - 60 秒窗口内最多允许 200 条客户端消息。
    pub message_window_started_at: Instant,
    pub message_count_in_window: u16,
}

impl Default for WsSession {
    fn default() -> Self {
        Self {
            current_scope_nonce: 0,
            current_sync_scope_nonce: None,
            authenticated_peer_id: None,
            bound_repo_id: None,
            writer_identity: None,
            browser_session: false,
            active_branch: None,
            active_repo: None,
            active_repo_id: None,
            active_db: None,
            message_window_started_at: Instant::now(),
            message_count_in_window: 0,
        }
    }
}

#[allow(dead_code)] // 为 P2P 握手和分支切换预留
impl WsSession {
    /// 创建新会话
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置已认证的 Peer ID
    pub fn set_authenticated(&mut self, peer_id: PeerId) {
        if self.authenticated_peer_id.as_ref() != Some(&peer_id) {
            self.writer_identity = None;
        }
        self.authenticated_peer_id = Some(peer_id);
    }

    pub fn mark_browser_session(&mut self) {
        self.browser_session = true;
    }

    pub fn is_browser_session(&self) -> bool {
        self.browser_session
    }

    /// 绑定仓库 ID (在握手成功后调用)
    pub fn bind_repo(&mut self, repo_id: RepoId) {
        if self.bound_repo_id != Some(repo_id) {
            self.writer_identity = None;
        }
        self.bound_repo_id = Some(repo_id);
    }

    pub fn set_writer_identity(&mut self, repo_id: RepoId, peer_id: PeerId) {
        self.writer_identity = Some(WriterIdentity { peer_id, repo_id });
    }

    pub fn writer_peer_id_for(&self, repo_id: &RepoId) -> Option<PeerId> {
        self.writer_identity
            .as_ref()
            .filter(|writer| &writer.repo_id == repo_id)
            .map(|writer| writer.peer_id.clone())
    }

    pub fn clear_sync_binding(&mut self) {
        self.authenticated_peer_id = None;
        self.bound_repo_id = None;
        self.writer_identity = None;
        self.current_sync_scope_nonce = None;
    }

    /// 检查给定 repo_id 是否与会话绑定一致
    pub fn is_repo_bound(&self, repo_id: &RepoId) -> bool {
        self.bound_repo_id.as_ref() == Some(repo_id)
    }

    /// 切换活动分支
    ///
    /// 传入 `None` 切换回本地分支，传入 `Some(id)` 切换到影子库。
    pub fn switch_branch(&mut self, peer_id: Option<String>) {
        self.active_branch = peer_id.map(PeerId::new);
    }

    pub fn set_scope_nonce(&mut self, scope_nonce: Option<u64>) {
        if let Some(scope_nonce) = scope_nonce {
            self.current_scope_nonce = scope_nonce;
        }
    }

    pub fn scope_nonce(&self) -> u64 {
        self.current_scope_nonce
    }

    pub fn set_sync_scope_nonce(&mut self, scope_nonce: u64) {
        self.current_sync_scope_nonce = Some(scope_nonce);
    }

    pub fn sync_scope_nonce(&self) -> Option<u64> {
        self.current_sync_scope_nonce
    }

    /// 切换活动仓库
    pub fn switch_repo(&mut self, repo_name: String, repo_id: Option<RepoId>) {
        self.active_repo = Some(repo_name);
        self.active_repo_id = repo_id;
    }

    /// 设置活动数据库句柄
    pub fn set_active_db(&mut self, handle: DatabaseHandle) {
        self.active_db = Some(handle);
    }

    pub fn clear_active_db(&mut self) {
        self.active_db = None;
    }

    pub fn clear_active_repo(&mut self) {
        self.active_repo = None;
        self.active_repo_id = None;
    }

    /// 检查是否在影子分支 (只读模式)
    pub fn is_readonly(&self) -> bool {
        self.active_db.as_ref().map(|h| h.readonly).unwrap_or(false)
    }

    /// 获取活动数据库引用 (如果已锁定)
    pub fn get_active_db(&self) -> Option<&DatabaseHandle> {
        self.active_db.as_ref()
    }

    pub fn active_db_for(
        &self,
        branch: Option<&PeerId>,
        repo_name: &str,
        repo_id: Option<RepoId>,
    ) -> Option<&DatabaseHandle> {
        self.active_db.as_ref().filter(|handle| {
            handle.branch.as_ref() == branch && active_db_matches_scope(handle, repo_name, repo_id)
        })
    }

    pub fn record_incoming_message(&mut self, now: Instant) -> bool {
        if now.duration_since(self.message_window_started_at) >= WS_MESSAGE_WINDOW {
            self.message_window_started_at = now;
            self.message_count_in_window = 0;
        }

        if self.message_count_in_window >= WS_MAX_MESSAGES_PER_WINDOW {
            return false;
        }

        self.message_count_in_window += 1;
        true
    }
}

fn active_db_matches_scope(
    handle: &DatabaseHandle,
    repo_name: &str,
    repo_id: Option<RepoId>,
) -> bool {
    match (repo_id, handle.repo_id) {
        (Some(expected), Some(active)) => active == expected,
        (Some(_), None) => handle.repo_name.as_str() == repo_name,
        (None, _) => handle.repo_name.as_str() == repo_name,
    }
}

#[cfg(test)]
#[path = "session_test.rs"]
mod tests;
