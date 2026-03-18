use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
struct SessionBroadcastScope {
    active_repo_id: Option<RepoId>,
    active_branch: Option<PeerId>,
    scope_nonce: u64,
}

impl SessionBroadcastScope {
    fn from_session(session: &WsSession) -> Self {
        Self {
            active_repo_id: session.active_repo_id,
            active_branch: session.active_branch.clone(),
            scope_nonce: session.scope_nonce(),
        }
    }
}

/// 广播过滤器：按连接的 repo/branch 视图裁剪广播消息。
///
/// Invariants:
/// - `FsChangeDetected` 只应投递到本地分支会话。
/// - 若会话已锁定 `active_repo_id`，则只接收同仓库事件。
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
                repo_id, branch, ..
            }
            | ServerMessage::CommitAck {
                repo_id, branch, ..
            }
            | ServerMessage::MergeComplete {
                repo_id, branch, ..
            } => matches_scope(
                scope.active_repo_id,
                scope.active_branch.as_ref(),
                repo_id,
                branch.as_ref(),
                true,
            ),
            ServerMessage::NewOp {
                repo_id, branch, ..
            } => matches_scope(
                scope.active_repo_id,
                scope.active_branch.as_ref(),
                &Some(*repo_id),
                branch.as_ref(),
                false,
            ),
            ServerMessage::PeerDeleted { .. } => scope.active_repo_id.is_some(),
            _ => true,
        }
    }

    pub(crate) fn stamp_scope_nonce(&self, msg: ServerMessage) -> Option<ServerMessage> {
        let Some(scope) = &self.scope else {
            return Some(msg);
        };
        let Ok(scope) = scope.read() else {
            tracing::error!("WS broadcast filter read lock poisoned during nonce stamp; dropping broadcast");
            return None;
        };

        Some(match msg {
            ServerMessage::FsChangeDetected {
                repo_id,
                branch,
                path,
                change_type,
                has_conflict,
                ..
            } => ServerMessage::FsChangeDetected {
                repo_id,
                branch,
                scope_nonce: Some(scope.scope_nonce),
                path,
                change_type,
                has_conflict,
            },
            ServerMessage::CommitAck {
                repo_id,
                branch,
                commit_id,
                timestamp,
                ..
            } => ServerMessage::CommitAck {
                repo_id,
                branch,
                scope_nonce: Some(scope.scope_nonce),
                commit_id,
                timestamp,
            },
            ServerMessage::NewOp {
                repo_id,
                branch,
                doc_id,
                entry,
                ..
            } => ServerMessage::NewOp {
                repo_id,
                branch,
                scope_nonce: Some(scope.scope_nonce),
                doc_id,
                entry,
            },
            ServerMessage::MergeComplete {
                repo_id,
                branch,
                merged_count,
                ..
            } => ServerMessage::MergeComplete {
                repo_id,
                branch,
                scope_nonce: Some(scope.scope_nonce),
                merged_count,
            },
            ServerMessage::PeerDeleted { peer_id, .. } => ServerMessage::PeerDeleted {
                peer_id,
                scope_nonce: Some(scope.scope_nonce),
            },
            other => other,
        })
    }

    pub(crate) fn current_scope_nonce(&self) -> Option<u64> {
        let scope = self.scope.as_ref()?;
        let scope = match scope.read() {
            Ok(scope) => scope,
            Err(_) => {
                tracing::error!(
                    "WS broadcast filter read lock poisoned while reading current scope nonce"
                );
                return None;
            }
        };
        Some(scope.scope_nonce)
    }
}

fn matches_scope(
    active_repo_id: Option<RepoId>,
    active_branch: Option<&PeerId>,
    message_repo_id: &Option<RepoId>,
    message_branch: Option<&PeerId>,
    local_only: bool,
) -> bool {
    if local_only && active_branch.is_some() {
        return false;
    }
    match (active_repo_id, message_repo_id, message_branch) {
        (None, Some(_), _) => false,
        (Some(_), None, _) => false,
        (Some(active_repo_id), Some(message_repo_id), Some(branch)) => {
            active_repo_id == *message_repo_id && active_branch == Some(branch)
        }
        (Some(active_repo_id), Some(message_repo_id), None) => {
            active_repo_id == *message_repo_id && active_branch.is_none()
        }
        _ => true,
    }
}

#[cfg(test)]
#[path = "filter_test.rs"]
mod tests;
