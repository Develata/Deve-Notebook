use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
struct SessionBroadcastScope {
    active_repo_id: Option<RepoId>,
    active_branch: Option<PeerId>,
}

impl SessionBroadcastScope {
    fn from_session(session: &WsSession) -> Self {
        Self {
            active_repo_id: session.active_repo_id,
            active_branch: session.active_branch.clone(),
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
            | ServerMessage::MergeComplete { repo_id, .. } => matches_scope(
                scope.active_repo_id,
                scope.active_branch.as_ref(),
                repo_id,
                None,
                true,
            ),
            ServerMessage::NewOp {
                repo_id, branch, ..
            } => matches_scope(
                Some(*repo_id),
                scope.active_branch.as_ref(),
                &Some(*repo_id),
                branch.as_ref(),
                false,
            ),
            _ => true,
        }
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
mod tests {
    use super::BroadcastFilter;
    use crate::server::session::WsSession;
    use deve_core::models::{DocId, Op, PeerId};
    use deve_core::protocol::{ConfirmedOp, ServerMessage};

    #[test]
    fn rejects_new_op_from_other_branch() {
        let mut session = WsSession::new();
        session.switch_branch(Some("peer-a".into()));
        session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
        let filter = BroadcastFilter::for_session(&session);

        assert!(!filter.should_forward(&ServerMessage::NewOp {
            repo_id: uuid::Uuid::nil(),
            branch: Some(PeerId::new("peer-b")),
            doc_id: DocId::new(),
            entry: ConfirmedOp::new(
                1,
                Op::Insert {
                    pos: 0,
                    content: "x".into()
                },
                None
            ),
        }));
    }
}
