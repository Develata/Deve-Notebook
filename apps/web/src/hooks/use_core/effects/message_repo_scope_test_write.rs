use super::*;

#[test]
fn rejects_write_ready_while_repo_switch_pending() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!accepts_write_ready(
        &repo_id,
        &None,
        3,
        Some(repo_id.clone()),
        None,
        Some(PendingBranchTarget::Local),
        Some("default".into()),
        Some(3),
    ));
}

#[test]
fn rejects_write_ready_while_branch_switch_pending() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!accepts_write_ready(
        &repo_id,
        &None,
        3,
        Some(repo_id.clone()),
        None,
        Some(PendingBranchTarget::Local),
        None,
        Some(3),
    ));
}

#[test]
fn accepts_write_ready_only_for_local_branch_and_bound_repo() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(accepts_write_ready(
        &repo_id,
        &None,
        3,
        Some(repo_id.clone()),
        None,
        None,
        None,
        Some(3),
    ));
    assert!(!accepts_write_ready(
        &repo_id,
        &Some(PeerId::new("peer-a")),
        3,
        Some(repo_id.clone()),
        Some(PeerId::new("peer-a")),
        None,
        None,
        Some(3),
    ));
    assert!(!accepts_write_ready(
        &repo_id,
        &None,
        2,
        Some(repo_id.clone()),
        None,
        None,
        None,
        Some(3),
    ));
}
