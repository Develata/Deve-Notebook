use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerMessage};
use std::sync::{Arc, RwLock};

#[path = "filter_scope.rs"]
mod scope;
#[path = "filter_stamp.rs"]
mod stamp;

use scope::{SessionBroadcastScope, matches_runtime_scope_nonce, matches_scope};

/// 广播过滤器：按连接的 repo/branch 视图裁剪广播消息。
///
/// Invariants:
/// - `FsChangeDetected` 只应投递到本地分支会话。
/// - 若会话已锁定 `active_repo_id`，则只接收同仓库事件。
/// - `PeerDeleted` 属于浏览器控制面事件，只投递给 browser session。
#[derive(Clone, Default)]
pub(crate) struct BroadcastFilter {
    scope: Option<Arc<RwLock<SessionBroadcastScope>>>,
}

impl BroadcastFilter {
    pub(crate) fn allow_all() -> Self {
        Self::default()
    }

    pub(crate) fn for_session(session: &WsSession) -> Self {
        Self {
            scope: Some(Arc::new(RwLock::new(SessionBroadcastScope::from_session(
                session,
            )))),
        }
    }

    pub(crate) fn sync_from_session(&self, session: &WsSession) {
        let Some(scope) = &self.scope else {
            return;
        };
        match scope.write() {
            Ok(mut slot) => *slot = SessionBroadcastScope::from_session(session),
            Err(_) => {
                tracing::error!("WS broadcast filter write lock poisoned; keeping filter closed");
            }
        }
    }

    pub(crate) fn should_forward(&self, msg: &ServerMessage) -> bool {
        let Some(scope) = &self.scope else {
            return true;
        };
        let Ok(scope) = scope.read() else {
            tracing::error!("WS broadcast filter read lock poisoned; dropping broadcast");
            return false;
        };

        match msg {
            ServerMessage::FsChangeDetected {
                repo_id,
                branch,
                scope_nonce,
                ..
            }
            | ServerMessage::CommitAck {
                repo_id,
                branch,
                scope_nonce,
                ..
            }
            | ServerMessage::MergeComplete {
                repo_id,
                branch,
                scope_nonce,
                ..
            } => {
                matches_scope(
                    scope.active_repo_id,
                    scope.active_branch.as_ref(),
                    repo_id,
                    branch.as_ref(),
                    true,
                ) && matches_runtime_scope_nonce(scope.scope_nonce, *scope_nonce)
            }
            ServerMessage::NewOp {
                repo_id,
                branch,
                scope_nonce,
                ..
            } => {
                matches_scope(
                    scope.active_repo_id,
                    scope.active_branch.as_ref(),
                    &Some(*repo_id),
                    branch.as_ref(),
                    false,
                ) && matches_runtime_scope_nonce(scope.scope_nonce, *scope_nonce)
            }
            ServerMessage::PeerDeleted { scope_nonce, .. } => {
                scope.browser_session
                    && matches_runtime_scope_nonce(scope.scope_nonce, *scope_nonce)
            }
            _ => true,
        }
    }

    pub(crate) fn stamp_scope_nonce(&self, msg: ServerMessage) -> Option<ServerMessage> {
        let Some(scope) = &self.scope else {
            return Some(msg);
        };
        let Ok(scope) = scope.read() else {
            tracing::error!(
                "WS broadcast filter read lock poisoned during nonce stamp; dropping broadcast"
            );
            return None;
        };

        Some(stamp::stamp_scope_nonce(msg, scope.scope_nonce))
    }

    pub(crate) fn scoped_protocol_error(
        &self,
        error: ServerError,
        switch_nonce: Option<u64>,
    ) -> Option<ServerMessage> {
        let scope_nonce = match &self.scope {
            None => None,
            Some(scope) => match scope.read() {
                Ok(scope) => Some(scope.scope_nonce),
                Err(_) => {
                    tracing::error!(
                        "WS broadcast filter read lock poisoned while building scoped protocol error"
                    );
                    return None;
                }
            },
        };
        Some(stamp::scoped_protocol_error(
            error,
            switch_nonce,
            scope_nonce,
        ))
    }
}

#[cfg(test)]
#[path = "filter_poison_test.rs"]
mod poison_tests;
#[cfg(test)]
#[path = "filter_test.rs"]
mod tests;
