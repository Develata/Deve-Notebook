use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

pub fn peer_branch_matches_scope(
    branch: &Option<PeerId>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> bool {
    *branch == expected_peer_branch(active_branch, pending_branch_switch)
}

pub fn string_branch_matches_scope(
    branch: &Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> bool {
    *branch == expected_branch_string(active_branch, pending_branch_switch)
}

pub fn repo_list_matches_scope(
    branch: Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    pending_repo_switch.is_none()
        && branch == expected_branch_string(active_branch, pending_branch_switch)
}

fn expected_branch_string(
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> Option<String> {
    expected_peer_branch(active_branch, pending_branch_switch).map(|peer_id| peer_id.to_string())
}

fn expected_peer_branch(
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> Option<PeerId> {
    pending_branch_switch
        .map(|pending| match pending {
            PendingBranchTarget::Local => None,
            PendingBranchTarget::Shadow(peer_id) => Some(PeerId::new(peer_id)),
        })
        .unwrap_or(active_branch)
}

#[cfg(test)]
mod tests {
    use super::{peer_branch_matches_scope, repo_list_matches_scope, string_branch_matches_scope};
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::PeerId;

    #[test]
    fn repo_list_uses_pending_branch_scope_during_switch() {
        assert!(!repo_list_matches_scope(
            None,
            None,
            Some(PendingBranchTarget::Shadow("peer-a".into())),
            None,
        ));
        assert!(repo_list_matches_scope(
            Some("peer-a".into()),
            None,
            Some(PendingBranchTarget::Shadow("peer-a".into())),
            None,
        ));
    }

    #[test]
    fn repo_list_uses_active_branch_without_pending_switch() {
        assert!(repo_list_matches_scope(
            Some("peer-a".into()),
            Some(PeerId::new("peer-a")),
            None,
            None,
        ));
        assert!(!repo_list_matches_scope(
            Some("peer-b".into()),
            Some(PeerId::new("peer-a")),
            None,
            None,
        ));
    }

    #[test]
    fn repo_list_rejects_messages_while_repo_switch_pending() {
        assert!(!repo_list_matches_scope(
            None,
            None,
            None,
            Some("default".into()),
        ));
    }

    #[test]
    fn peer_scope_prefers_pending_branch_target() {
        assert!(!peer_branch_matches_scope(
            &None,
            None,
            Some(PendingBranchTarget::Shadow("peer-a".into())),
        ));
        assert!(peer_branch_matches_scope(
            &Some(PeerId::new("peer-a")),
            None,
            Some(PendingBranchTarget::Shadow("peer-a".into())),
        ));
    }

    #[test]
    fn string_scope_accepts_pending_local_branch() {
        assert!(string_branch_matches_scope(
            &None,
            Some(PeerId::new("peer-a")),
            Some(PendingBranchTarget::Local),
        ));
        assert!(!string_branch_matches_scope(
            &Some("peer-a".into()),
            Some(PeerId::new("peer-a")),
            Some(PendingBranchTarget::Local),
        ));
    }
}
