use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::PeerId;
use tempfile::TempDir;

fn read_repo_info_exists(db: &redb::Database) -> bool {
    let read = db.begin_read().expect("read txn");
    let table = read.open_table(REPO_METADATA).expect("repo metadata");
    table.get(&0).expect("read metadata").is_some()
}

#[test]
fn runtime_remote_listing_does_not_write_back_legacy_shadow_metadata() {
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
    let handle = repo
        .open_database(Some(&peer_id), &info.uuid.to_string())
        .expect("open legacy shadow");
    assert!(!read_repo_info_exists(handle.db.as_ref()));

    assert_eq!(
        repo.list_repos(Some(&peer_id))
            .expect("list legacy shadow without repair"),
        vec![info.uuid.to_string()]
    );
    assert!(!read_repo_info_exists(handle.db.as_ref()));
}

#[test]
fn runtime_remote_selector_recovery_does_not_write_back_legacy_shadow_metadata() {
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
    let handle = repo
        .open_database(Some(&peer_id), &info.uuid.to_string())
        .expect("open legacy shadow");
    assert!(!read_repo_info_exists(handle.db.as_ref()));

    assert_eq!(
        repo.find_remote_repo_selector_by_id(&peer_id, info.uuid)
            .expect("resolve selector"),
        Some(info.uuid.to_string())
    );
    assert!(!read_repo_info_exists(handle.db.as_ref()));
}
