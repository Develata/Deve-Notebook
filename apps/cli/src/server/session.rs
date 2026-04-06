// apps/cli/src/server/session.rs
//! # WebSocket 会话状态 (Session State)
//!
//! 管理单个 WebSocket 连接的 repo-scoped 会话状态。

use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::{PeerId, RepoId};
use std::time::{Duration, Instant};

const WS_MESSAGE_WINDOW: Duration = Duration::from_secs(60);
const WS_MAX_MESSAGES_PER_WINDOW: u16 = 200;

#[path = "session_writer.rs"]
mod writer;

pub use writer::WriterIdentity;

/// WebSocket 会话状态
///
/// 每个 WebSocket 连接维护独立的会话状态实例。
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

    /// 已认证的对端 Peer ID，用于后续 SyncPush 验证。
    pub authenticated_peer_id: Option<PeerId>,

    /// 当前绑定的仓库 ID，用于后续同步消息的 repo 一致性校验。
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

    /// 最近一次稳定绑定的本地仓库 hint。
    ///
    /// Invariant:
    /// - 仅在 `active_branch == None` 的本地 scope 下更新。
    /// - 远端分支临时切仓不得覆盖该 hint，避免切回 Local 时丢失用户原本的本地 repo。
    pub last_local_repo: Option<String>,
    pub last_local_repo_id: Option<RepoId>,

    /// 当前锁定的数据库句柄。
    pub active_db: Option<DatabaseHandle>,

    /// WebSocket 固定时间窗限流状态。
    ///
    /// Invariant:
    /// - 每个连接独立计数。
    /// - 60 秒窗口内最多允许 200 条客户端消息。
    pub message_window_started_at: Instant,
    pub message_count_in_window: u16,
}

#[path = "session_repo.rs"]
mod session_repo;
#[path = "session_scope.rs"]
mod session_scope;

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
            last_local_repo: None,
            last_local_repo_id: None,
            active_db: None,
            message_window_started_at: Instant::now(),
            message_count_in_window: 0,
        }
    }
}

impl WsSession {
    /// 创建新会话
    pub fn new() -> Self {
        Self::default()
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

#[cfg(test)]
#[path = "session_test.rs"]
mod tests;
