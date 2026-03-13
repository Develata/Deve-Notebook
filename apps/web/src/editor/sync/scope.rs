use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::{PeerId, RepoId};

#[derive(Clone)]
pub struct SyncPayloadScope {
    pub current_repo_id: Option<String>,
    pub pending_repo_switch: Option<String>,
    pub current_branch: Option<PeerId>,
    pub pending_branch_switch: Option<PendingBranchTarget>,
    pub handshake_scope_nonce: Option<u64>,
}

pub fn matches_scope(
    current_repo_id: Option<String>,
    pending_repo_switch: Option<String>,
    current_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
) -> bool {
    if pending_repo_switch.is_some() || pending_branch_switch.is_some() {
        return false;
    }
    let expected_branch = pending_branch_switch
        .map(|pending| match pending {
            PendingBranchTarget::Local => None,
            PendingBranchTarget::Shadow(peer_id) => Some(PeerId::new(peer_id)),
        })
        .unwrap_or(current_branch);
    match (repo_id, current_repo_id) {
        (Some(repo_id), Some(current)) => {
            current == repo_id.to_string() && branch == expected_branch
        }
        (Some(_), None) => false,
        (None, Some(_)) => false,
        (None, None) => branch == expected_branch,
    }
}

pub fn accepts_sync_payload(
    scope: SyncPayloadScope,
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: u64,
) -> bool {
    scope.handshake_scope_nonce == Some(scope_nonce)
        && matches_scope(
            scope.current_repo_id,
            scope.pending_repo_switch,
            scope.current_branch,
            scope.pending_branch_switch,
            Some(repo_id),
            branch,
        )
}

#[cfg(test)]
mod tests {
    use super::{SyncPayloadScope, accepts_sync_payload, matches_scope};
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::PeerId;

    #[test]
    fn matches_scope_rejects_same_repo_on_different_branch() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(!matches_scope(
            Some(repo_id.to_string()),
            None,
            Some(PeerId::new("peer-b")),
            None,
            Some(repo_id),
            Some(PeerId::new("peer-a")),
        ));
    }

    #[test]
    fn matches_scope_accepts_same_repo_and_branch() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(matches_scope(
            Some(repo_id.to_string()),
            None,
            Some(PeerId::new("peer-a")),
            None,
            Some(repo_id),
            Some(PeerId::new("peer-a")),
        ));
    }

    #[test]
    fn matches_scope_rejects_same_repo_without_branch_when_remote_active() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(!matches_scope(
            Some(repo_id.to_string()),
            None,
            Some(PeerId::new("peer-a")),
            None,
            Some(repo_id),
            None,
        ));
    }

    #[test]
    fn matches_scope_rejects_repo_less_message_once_repo_is_bound() {
        assert!(!matches_scope(
            Some(uuid::Uuid::new_v4().to_string()),
            None,
            None,
            None,
            None,
            None,
        ));
    }

    #[test]
    fn matches_scope_accepts_repo_less_message_before_repo_binding() {
        assert!(matches_scope(None, None, None, None, None, None));
    }

    #[test]
    fn matches_scope_rejects_messages_while_repo_switch_pending() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(!matches_scope(
            Some(repo_id.to_string()),
            Some("test".into()),
            None,
            None,
            Some(repo_id),
            None,
        ));
    }

    #[test]
    fn matches_scope_rejects_messages_while_branch_switch_pending() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(!matches_scope(
            Some(repo_id.to_string()),
            None,
            Some(PeerId::new("peer-a")),
            Some(PendingBranchTarget::Local),
            Some(repo_id),
            None,
        ));
    }

    #[test]
    fn matches_scope_prefers_pending_branch_target() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(!matches_scope(
            Some(repo_id.to_string()),
            None,
            Some(PeerId::new("peer-a")),
            Some(PendingBranchTarget::Shadow("peer-b".into())),
            Some(repo_id),
            Some(PeerId::new("peer-b")),
        ));
    }

    #[test]
    fn accepts_sync_payload_only_for_current_handshake_scope() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(accepts_sync_payload(
            SyncPayloadScope {
                current_repo_id: Some(repo_id.to_string()),
                pending_repo_switch: None,
                current_branch: None,
                pending_branch_switch: None,
                handshake_scope_nonce: Some(5),
            },
            repo_id,
            None,
            5,
        ));
        assert!(!accepts_sync_payload(
            SyncPayloadScope {
                current_repo_id: Some(repo_id.to_string()),
                pending_repo_switch: None,
                current_branch: None,
                pending_branch_switch: None,
                handshake_scope_nonce: Some(6),
            },
            repo_id,
            None,
            5,
        ));
    }

    #[test]
    fn rejects_sync_payload_while_scope_switch_is_pending() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(!accepts_sync_payload(
            SyncPayloadScope {
                current_repo_id: Some(repo_id.to_string()),
                pending_repo_switch: Some("test".into()),
                current_branch: None,
                pending_branch_switch: None,
                handshake_scope_nonce: Some(5),
            },
            repo_id,
            None,
            5,
        ));
    }
}
