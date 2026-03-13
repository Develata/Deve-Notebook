use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::PeerId;
use tempfile::TempDir;

fn read_repo_info(db: &redb::Database) -> deve_core::ledger::RepoInfo {
    let read = db.begin_read().expect("read txn");
    let table = read.open_table(REPO_METADATA).expect("repo metadata");
    let raw = table.get(&0).expect("read metadata").expect("metadata row");
    bincode::deserialize(raw.value()).expect("deserialize repo info")
}

#[test]
fn remote_catalog_repairs_legacy_uuid_shadow_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let local = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("wiki"),
        Some("urn:test:wiki"),
    )
    .expect("init local companion");
    let info = local
        .get_repo_info()
        .expect("local repo info")
        .expect("local repo exists");
    let peer_id = PeerId::new("peer-remote");

    repo.ensure_shadow_db(&peer_id, &info.uuid)
        .expect("create legacy uuid shadow");
    assert_eq!(
        repo.list_repos(Some(&peer_id))
            .expect("list repaired shadows"),
        vec!["wiki".to_string()]
    );

    let handle = repo
        .open_database(Some(&peer_id), "wiki")
        .expect("open repaired shadow");
    let repaired = read_repo_info(&handle.db);
    assert_eq!(repaired.uuid, info.uuid);
    assert_eq!(repaired.name, "wiki");
    assert_eq!(repaired.url.as_deref(), Some("urn:test:wiki"));
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join("wiki.redb")
            .exists()
    );
}
