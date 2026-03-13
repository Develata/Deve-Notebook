use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::schema::REPO_METADATA;
use deve_core::models::PeerId;
use tempfile::TempDir;

fn read_repo_info(db: &redb::Database) -> Option<deve_core::ledger::RepoInfo> {
    let read = db.begin_read().expect("read txn");
    let table = read.open_table(REPO_METADATA).expect("repo metadata");
    let raw = table.get(&0).expect("read metadata")?;
    Some(bincode::deserialize(raw.value()).expect("deserialize repo info"))
}

#[test]
fn remote_catalog_repairs_legacy_uuid_shadow_to_named_catalog() {
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
    let repaired = read_repo_info(&handle.db).expect("repo info written back");
    assert_eq!(repaired.name, "wiki");
    assert_eq!(repaired.uuid, info.uuid);
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join("wiki.redb")
            .exists()
    );
    assert!(
        !repo
            .remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}

#[test]
fn init_repairs_uuid_shadow_path_and_metadata() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let local = RepoManager::init(&ledger_dir, 10, Some("wiki"), Some("urn:test:wiki"))
        .expect("init local companion");
    let info = local
        .get_repo_info()
        .expect("local repo info")
        .expect("local repo exists");
    let peer_id = PeerId::new("peer-remote");

    repo.ensure_shadow_db(&peer_id, &info.uuid)
        .expect("create legacy uuid shadow");
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );

    let repaired = RepoManager::init(&ledger_dir, 10, None, None).expect("re-init repo");
    assert_eq!(
        repaired
            .list_repos(Some(&peer_id))
            .expect("list repaired shadows"),
        vec!["wiki".to_string()]
    );
    assert!(
        repaired
            .remotes_dir()
            .join(peer_id.to_filename())
            .join("wiki.redb")
            .exists()
    );
    assert!(
        !repaired
            .remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}

#[test]
fn local_catalog_repair_leaves_irrecoverable_legacy_uuid_shadow_as_uuid_selector() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 10, Some("main"), Some("urn:main")).expect("main");
    let wiki = RepoManager::init(&ledger_dir, 10, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let peer_id = PeerId::new("peer-remote");
    let wiki_info = wiki.get_repo_info().expect("wiki info").expect("present");

    main.ensure_shadow_db(&peer_id, &wiki_info.uuid)
        .expect("create legacy uuid shadow");

    let main_info = main.get_repo_info().expect("main info").expect("present");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;
    let bad = deve_core::ledger::RepoInfo {
        uuid: main_info.uuid,
        name: "main".into(),
        url: Some(format!("urn:uuid:{}", main_info.uuid)),
    };
    let write = wiki_db.begin_write().expect("write txn");
    write
        .open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(&bad)
                .expect("serialize repaired metadata")
                .as_slice(),
        )
        .expect("write bad metadata");
    write.commit().expect("commit metadata");

    main.repair_local_repo_catalog()
        .expect("repair local catalog");

    assert_eq!(
        main.list_repos(Some(&peer_id))
            .expect("list repaired shadows"),
        vec![wiki_info.uuid.to_string()]
    );
    assert!(
        main.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", wiki_info.uuid))
            .exists()
    );
}
