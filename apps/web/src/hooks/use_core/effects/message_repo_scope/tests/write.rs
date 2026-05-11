use super::*;

fn write_ready_input(repo_id: &str) -> WriteReadyScopeInput<'_> {
    WriteReadyScopeInput {
        repo_id,
        branch: None,
        scope_nonce: 3,
        current_repo_id: Some(repo_id.to_owned()),
        active_branch: None,
        pending_branch_switch: None,
        pending_repo_switch: None,
        handshake_scope_nonce: Some(3),
    }
}

#[test]
fn rejects_write_ready_while_repo_switch_pending() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!accepts_write_ready(WriteReadyScopeInput {
        pending_branch_switch: Some(PendingBranchTarget::Local),
        pending_repo_switch: Some("default".into()),
        ..write_ready_input(&repo_id)
    }));
}

#[test]
fn rejects_write_ready_while_branch_switch_pending() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!accepts_write_ready(WriteReadyScopeInput {
        pending_branch_switch: Some(PendingBranchTarget::Local),
        ..write_ready_input(&repo_id)
    }));
}

#[test]
fn accepts_write_ready_only_for_local_branch_and_bound_repo() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(accepts_write_ready(write_ready_input(&repo_id)));
    assert!(!accepts_write_ready(WriteReadyScopeInput {
        branch: Some(PeerId::new("peer-a")),
        active_branch: Some(PeerId::new("peer-a")),
        ..write_ready_input(&repo_id)
    }));
    assert!(!accepts_write_ready(WriteReadyScopeInput {
        scope_nonce: 2,
        ..write_ready_input(&repo_id)
    }));
}
