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
    request_id: Option<String>,
    branch: Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    expected_request_id: Option<String>,
) -> bool {
    request_matches(request_id.as_deref(), expected_request_id.as_deref())
        && pending_repo_switch.is_none()
        && pending_branch_switch.is_none()
        && branch == expected_branch_string(active_branch, pending_branch_switch)
}

pub fn shadow_list_matches_scope(
    request_id: Option<String>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    expected_request_id: Option<String>,
) -> bool {
    request_matches(request_id.as_deref(), expected_request_id.as_deref())
        && pending_branch_switch.is_none()
        && pending_repo_switch.is_none()
}

pub fn accepts_system_or_matching_request(
    message_id: Option<&str>,
    expected_id: Option<&str>,
) -> bool {
    match message_id {
        Some(message_id) => expected_id == Some(message_id),
        None => true,
    }
}

fn request_matches(message_id: Option<&str>, expected_id: Option<&str>) -> bool {
    match message_id {
        Some(message_id) => expected_id == Some(message_id),
        None => expected_id.is_none(),
    }
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
    use super::{
        accepts_system_or_matching_request, peer_branch_matches_scope, repo_list_matches_scope,
        shadow_list_matches_scope, string_branch_matches_scope,
    };
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::PeerId;

    #[test]
    fn repo_list_rejects_messages_while_branch_switch_pending() {
        assert!(!repo_list_matches_scope(
            None,
            None,
            None,
            Some(PendingBranchTarget::Shadow("peer-a".into())),
            None,
            None,
        ));
    }

    #[test]
    fn repo_list_uses_active_branch_without_pending_switch() {
        assert!(repo_list_matches_scope(
            None,
            Some("peer-a".into()),
            Some(PeerId::new("peer-a")),
            None,
            None,
            None,
        ));
        assert!(!repo_list_matches_scope(
            None,
            Some("peer-b".into()),
            Some(PeerId::new("peer-a")),
            None,
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
            None,
            Some("default".into()),
            None,
        ));
    }

    #[test]
    fn shadow_list_rejects_messages_while_switch_pending() {
        assert!(!shadow_list_matches_scope(
            None,
            Some(PendingBranchTarget::Shadow("peer-a".into())),
            None,
            None,
        ));
        assert!(!shadow_list_matches_scope(
            None,
            None,
            Some("default".into()),
            None,
        ));
        assert!(shadow_list_matches_scope(None, None, None, None));
    }

    #[test]
    fn scoped_list_accepts_matching_request_id_only() {
        assert!(shadow_list_matches_scope(
            Some("req-1".into()),
            None,
            None,
            Some("req-1".into()),
        ));
        assert!(!shadow_list_matches_scope(
            Some("stale".into()),
            None,
            None,
            Some("req-1".into()),
        ));
    }

    #[test]
    fn system_or_matching_request_accepts_none_and_exact_match() {
        assert!(accepts_system_or_matching_request(None, Some("req-1")));
        assert!(accepts_system_or_matching_request(
            Some("req-1"),
            Some("req-1"),
        ));
        assert!(!accepts_system_or_matching_request(
            Some("stale"),
            Some("req-1"),
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
