use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefreshScope {
    repo_id: Option<String>,
    branch: Option<PeerId>,
}

pub fn capture_refresh_scope(
    repo_id: Option<String>,
    branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> Option<RefreshScope> {
    if pending_branch_switch.is_some() || pending_repo_switch.is_some() {
        return None;
    }
    Some(RefreshScope { repo_id, branch })
}

pub fn should_send_refresh(
    scope: &RefreshScope,
    repo_id: Option<String>,
    branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    pending_branch_switch.is_none()
        && pending_repo_switch.is_none()
        && scope.repo_id == repo_id
        && scope.branch == branch
}

pub fn can_issue_sc_refresh(pending_changes_request_id: Option<String>) -> bool {
    pending_changes_request_id.is_none()
}

#[cfg(test)]
mod tests {
    use super::{RefreshScope, can_issue_sc_refresh, capture_refresh_scope, should_send_refresh};
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::PeerId;

    #[test]
    fn does_not_capture_refresh_scope_during_switch() {
        assert_eq!(
            capture_refresh_scope(
                Some("repo-a".into()),
                None,
                Some(PendingBranchTarget::Local),
                None,
            ),
            None,
        );
    }

    #[test]
    fn rejects_refresh_after_repo_scope_changes() {
        let scope = RefreshScope {
            repo_id: Some("repo-a".into()),
            branch: Some(PeerId::new("peer-a")),
        };
        assert!(!should_send_refresh(
            &scope,
            Some("repo-b".into()),
            Some(PeerId::new("peer-a")),
            None,
            None,
        ));
        assert!(!should_send_refresh(
            &scope,
            Some("repo-a".into()),
            Some(PeerId::new("peer-b")),
            None,
            None,
        ));
    }

    #[test]
    fn keeps_refresh_only_when_scope_is_unchanged() {
        let scope = RefreshScope {
            repo_id: Some("repo-a".into()),
            branch: None,
        };
        assert!(should_send_refresh(
            &scope,
            Some("repo-a".into()),
            None,
            None,
            None,
        ));
    }

    #[test]
    fn sc_refresh_waits_for_inflight_request_to_finish() {
        assert!(!can_issue_sc_refresh(Some("req-1".into())));
        assert!(can_issue_sc_refresh(None));
    }
}
