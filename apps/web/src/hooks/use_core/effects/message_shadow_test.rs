use super::{should_recover_local_branch_from_shadow_list, should_refresh_shadow_list};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

#[test]
fn peer_deleted_only_refreshes_shadows_when_scope_is_stable() {
    assert!(should_refresh_shadow_list(None, None, false));
    assert!(!should_refresh_shadow_list(
        Some(PendingBranchTarget::Shadow("peer-a".into())),
        None,
        false,
    ));
    assert!(!should_refresh_shadow_list(
        None,
        Some("default".into()),
        false
    ));
    assert!(!should_refresh_shadow_list(None, None, true));
}

#[test]
fn shadow_list_recovers_local_only_when_current_peer_disappears() {
    assert!(should_recover_local_branch_from_shadow_list(
        &["peer-b".into()],
        Some(PeerId::new("peer-a")),
        None,
    ));
    assert!(!should_recover_local_branch_from_shadow_list(
        &["peer-a".into(), "peer-b".into()],
        Some(PeerId::new("peer-a")),
        None,
    ));
    assert!(!should_recover_local_branch_from_shadow_list(
        &[],
        None,
        None,
    ));
    assert!(!should_recover_local_branch_from_shadow_list(
        &["peer-b".into()],
        Some(PeerId::new("peer-a")),
        Some(PendingBranchTarget::Local),
    ));
}
