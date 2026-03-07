use crate::server::session::WsSession;
use deve_core::models::RepoId;
use deve_core::protocol::ServerMessage;
use std::sync::{Arc, RwLock};

#[derive(Clone, Copy, Default)]
struct SessionBroadcastScope {
    active_repo_id: Option<RepoId>,
    on_remote_branch: bool,
}

impl SessionBroadcastScope {
    fn from_session(session: &WsSession) -> Self {
        Self {
            active_repo_id: session.active_repo_id,
            on_remote_branch: session.active_branch.is_some(),
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
        if let Ok(mut slot) = scope.write() {
            *slot = SessionBroadcastScope::from_session(session);
        }
    }

    pub(crate) fn should_forward(&self, msg: &ServerMessage) -> bool {
        let Some(scope) = &self.scope else {
            return true;
        };
        let Ok(scope) = scope.read() else {
            return true;
        };

        match msg {
            ServerMessage::FsChangeDetected { repo_id, .. }
            | ServerMessage::CommitAck { repo_id, .. }
            | ServerMessage::MergeComplete { repo_id, .. } => {
                matches_repo(scope.active_repo_id, scope.on_remote_branch, repo_id)
            }
            _ => true,
        }
    }
}

fn matches_repo(
    active_repo_id: Option<RepoId>,
    on_remote_branch: bool,
    message_repo_id: &Option<RepoId>,
) -> bool {
    if on_remote_branch {
        return false;
    }
    match (active_repo_id, message_repo_id) {
        (Some(active_repo_id), Some(message_repo_id)) => active_repo_id == *message_repo_id,
        _ => true,
    }
}
