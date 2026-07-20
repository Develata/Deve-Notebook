use super::{
    should_recover_local_branch_from_deleted_peer, should_recover_local_branch_from_shadow_list,
    should_refresh_shadow_list,
};
use crate::hooks::use_core::{PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch};
use deve_core::models::PeerId;

#[test]
fn peer_deleted_only_refreshes_shadows_when_scope_is_stable() {
    assert!(should_refresh_shadow_list(None, None, false));
    assert!(!should_refresh_shadow_list(
        Some(PendingBranchSwitch::new(
            PendingBranchTarget::Shadow("peer-a".into()),
            1,
        )),
        None,
        false,
    ));
    assert!(!should_refresh_shadow_list(
        None,
        Some(PendingRepoSwitch::switch("default", uuid::Uuid::nil(), 1,)),
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
        None,
        true,
    ));
    assert!(!should_recover_local_branch_from_shadow_list(
        &["peer-a".into(), "peer-b".into()],
        Some(PeerId::new("peer-a")),
        None,
        None,
        true,
    ));
    assert!(!should_recover_local_branch_from_shadow_list(
        &[],
        None,
        None,
        None,
        true,
    ));
    assert!(!should_recover_local_branch_from_shadow_list(
        &["peer-b".into()],
        Some(PeerId::new("peer-a")),
        Some(PendingBranchSwitch::new(PendingBranchTarget::Local, 1)),
        None,
        true,
    ));
    assert!(!should_recover_local_branch_from_shadow_list(
        &["peer-b".into()],
        Some(PeerId::new("peer-a")),
        None,
        Some(PendingRepoSwitch::switch("default", uuid::Uuid::nil(), 1,)),
        true,
    ));
    assert!(!should_recover_local_branch_from_shadow_list(
        &["peer-b".into()],
        Some(PeerId::new("peer-a")),
        None,
        None,
        false,
    ));
}

#[test]
fn peer_deleted_recovers_local_only_for_active_shadow_branch() {
    assert!(should_recover_local_branch_from_deleted_peer(
        &PeerId::new("peer-a"),
        Some(PeerId::new("peer-a")),
        None,
        None,
    ));
    assert!(!should_recover_local_branch_from_deleted_peer(
        &PeerId::new("peer-b"),
        Some(PeerId::new("peer-a")),
        None,
        None,
    ));
    assert!(!should_recover_local_branch_from_deleted_peer(
        &PeerId::new("peer-a"),
        Some(PeerId::new("peer-a")),
        Some(PendingBranchSwitch::new(PendingBranchTarget::Local, 1)),
        None,
    ));
    assert!(!should_recover_local_branch_from_deleted_peer(
        &PeerId::new("peer-a"),
        Some(PeerId::new("peer-a")),
        None,
        Some(PendingRepoSwitch::switch("default", uuid::Uuid::nil(), 1,)),
    ));
}
