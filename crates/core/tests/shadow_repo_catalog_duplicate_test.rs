use deve_core::ledger::schema::REPO_METADATA;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;
use redb::Database;
use tempfile::TempDir;

fn seed_shadow_file(repo: &RepoManager, peer_id: &PeerId, stem: &str, info: &RepoInfo) {
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("peer dir");
    let path = peer_dir.join(format!("{stem}.redb"));
    let db = Database::create(&path).expect("shadow db");
    let write = db.begin_write().expect("write txn");
    write
        .open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(info)
                .expect("serialize repo info")
                .as_slice(),
        )
        .expect("write repo info");
    write.commit().expect("commit repo info");
}

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

    seed_shadow_file(&repo, &peer_id, "wiki", &info);
    seed_shadow_file(&repo, &peer_id, "wiki-1", &info);

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

    seed_shadow_file(&repo, &peer_id, "wiki", &info);
    seed_shadow_file(&repo, &peer_id, "wiki-1", &info);

    let err = RepoManager::init(&ledger_dir, 10, None, None)
        .err()
        .expect("duplicate shadow uuid must fail init");
    let detail = format!("{err:#}");
    assert!(detail.contains("Failed to repair remote repo catalogs during init"));
    assert!(detail.contains("Broken shadow peer peer-dup"));
    assert!(detail.contains("duplicate remote repository UUIDs"));
}
