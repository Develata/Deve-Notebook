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
                Some(*repo_id),
                scope.active_branch.as_ref(),
                &Some(*repo_id),
                branch.as_ref(),
                false,
            ),
            _ => true,
        }
    }

    pub(crate) fn stamp_scope_nonce(&self, msg: ServerMessage) -> ServerMessage {
        let Some(scope) = &self.scope else {
            return msg;
        };
        let Ok(scope) = scope.read() else {
            return msg;
        };

        match msg {
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
            other => other,
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

    #[test]
    fn stamps_runtime_broadcasts_with_session_scope_nonce() {
        let mut session = WsSession::new();
        session.set_scope_nonce(Some(9));
        let filter = BroadcastFilter::for_session(&session);

        let commit = filter.stamp_scope_nonce(ServerMessage::CommitAck {
            repo_id: None,
            branch: None,
            scope_nonce: None,
            commit_id: "c1".into(),
            timestamp: 1,
        });
        let fs = filter.stamp_scope_nonce(ServerMessage::FsChangeDetected {
            repo_id: None,
            branch: None,
            scope_nonce: None,
            path: "notes/a.md".into(),
            change_type: "modified".into(),
            has_conflict: false,
        });
        let merge = filter.stamp_scope_nonce(ServerMessage::MergeComplete {
            repo_id: None,
            branch: None,
            scope_nonce: None,
            merged_count: 2,
        });

        match commit {
            ServerMessage::CommitAck { scope_nonce, .. } => assert_eq!(scope_nonce, Some(9)),
            other => panic!("unexpected commit message: {:?}", other),
        }
        match fs {
            ServerMessage::FsChangeDetected { scope_nonce, .. } => {
                assert_eq!(scope_nonce, Some(9))
            }
            other => panic!("unexpected fs message: {:?}", other),
        }
        match merge {
            ServerMessage::MergeComplete { scope_nonce, .. } => {
                assert_eq!(scope_nonce, Some(9))
            }
            other => panic!("unexpected merge message: {:?}", other),
        }
    }
}
