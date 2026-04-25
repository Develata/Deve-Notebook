use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use tempfile::TempDir;

mod common;

#[test]
fn remote_catalog_repair_fails_closed_on_metadata_less_non_uuid_shadow() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-remote");
    common::seed_metadata_less_shadow_repo(&repo, &peer_id, "notes");

    let err = repo
        .repair_remote_repo_catalogs()
        .expect_err("metadata-less non-uuid shadow must fail closed");
    let detail = format!("{err:#}");
    assert!(detail.contains("Broken shadow peer peer-remote while repairing catalogs"));
    assert!(detail.contains("Broken shadow repo notes for peer peer-remote"));
    assert!(detail.contains("repo metadata missing and file stem is not a UUID"));
}

#[test]
fn init_fails_closed_on_metadata_less_non_uuid_shadow() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-remote");
    common::seed_metadata_less_shadow_repo(&repo, &peer_id, "notes");

    let err = RepoManager::init(&ledger_dir, 10, None, None)
        .err()
        .expect("metadata-less non-uuid shadow must fail init");
    let detail = format!("{err:#}");
    assert!(detail.contains("Failed to repair remote repo catalogs during init"));
    assert!(detail.contains("Broken shadow repo notes for peer peer-remote"));
}
