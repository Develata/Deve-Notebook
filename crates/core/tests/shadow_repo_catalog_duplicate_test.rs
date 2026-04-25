use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use tempfile::TempDir;

mod common;

#[test]
fn remote_catalog_repair_fails_closed_on_duplicate_shadow_uuid() {
    let dir = TempDir::new().expect("tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-dup");
    let info = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };

    common::seed_shadow_repo_info(&repo, &peer_id, "wiki", &info);
    common::seed_shadow_repo_info(&repo, &peer_id, "wiki-1", &info);

    let err = repo
        .repair_remote_repo_catalogs()
        .expect_err("duplicate shadow uuid must fail repair");
    let detail = format!("{err:#}");
    assert!(detail.contains("Broken shadow peer peer-dup"));
    assert!(detail.contains("duplicate remote repository UUIDs"));
}

#[test]
fn init_fails_closed_on_duplicate_shadow_uuid_catalogs() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-dup");
    let info = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };

    common::seed_shadow_repo_info(&repo, &peer_id, "wiki", &info);
    common::seed_shadow_repo_info(&repo, &peer_id, "wiki-1", &info);

    let err = RepoManager::init(&ledger_dir, 10, None, None)
        .err()
        .expect("duplicate shadow uuid must fail init");
    let detail = format!("{err:#}");
    assert!(detail.contains("Failed to repair remote repo catalogs during init"));
    assert!(detail.contains("Broken shadow peer peer-dup"));
    assert!(detail.contains("duplicate remote repository UUIDs"));
}
