use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use tempfile::TempDir;
use uuid::Uuid;

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    (dir, repo)
}

#[test]
fn remote_repo_selector_by_name_fails_closed_on_name_collision() {
    let (_dir, repo) = new_repo();
    let peer_id = PeerId::new("peer-remote");
    common::seed_shadow_repo_info(
        &repo,
        &peer_id,
        "shadow-a",
        &RepoInfo {
            uuid: Uuid::new_v4(),
            name: "wiki/raw".into(),
            url: Some("urn:test:wiki-a".into()),
        },
    );
    common::seed_shadow_repo_info(
        &repo,
        &peer_id,
        "shadow-b",
        &RepoInfo {
            uuid: Uuid::new_v4(),
            name: "wiki/raw".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    );

    let err = repo
        .find_remote_repo_selector(&peer_id, "wiki/raw")
        .expect_err("duplicate display name must fail closed");
    assert!(
        err.to_string()
            .contains("ambiguous remote repository selector")
    );
}
