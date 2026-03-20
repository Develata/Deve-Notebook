use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use redb::Database;
use tempfile::TempDir;

fn seed_metadata_less_shadow(repo: &RepoManager, peer_id: &PeerId, stem: &str) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let path = peer_dir.join(format!("{}.redb", stem));
    Database::create(&path).expect("shadow db");
}

#[test]
fn remote_catalog_repair_fails_closed_on_metadata_less_non_uuid_shadow() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-remote");
    seed_metadata_less_shadow(&repo, &peer_id, "notes");

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
    seed_metadata_less_shadow(&repo, &peer_id, "notes");

    let err = RepoManager::init(&ledger_dir, 10, None, None)
        .err()
        .expect("metadata-less non-uuid shadow must fail init");
    let detail = format!("{err:#}");
    assert!(detail.contains("Failed to repair remote repo catalogs during init"));
    assert!(detail.contains("Broken shadow repo notes for peer peer-remote"));
}
