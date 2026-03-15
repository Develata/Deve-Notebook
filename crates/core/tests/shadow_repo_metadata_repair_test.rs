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
fn remote_catalog_keeps_legacy_uuid_shadow_as_uuid_selector_without_remote_metadata() {
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
        vec![info.uuid.to_string()]
    );

    let handle = repo
        .open_database(Some(&peer_id), &info.uuid.to_string())
        .expect("open repaired shadow");
    let repaired = read_repo_info(&handle.db).expect("repo info written back");
    assert_eq!(repaired.name, info.uuid.to_string());
    assert_eq!(repaired.uuid, info.uuid);
    assert!(
        repo.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}

#[test]
fn init_keeps_uuid_shadow_path_without_remote_metadata() {
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
        vec![info.uuid.to_string()]
    );
    assert!(
        repaired
            .remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", info.uuid))
            .exists()
    );
}

#[test]
fn init_survives_broken_shadow_catalogs() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None, None).expect("init repo");
    let peer_id = PeerId::new("peer-bad");
    let peer_dir = repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir).expect("create peer dir");
    std::fs::write(peer_dir.join("broken.redb"), b"not-a-redb").expect("write broken shadow");

    let repaired = RepoManager::init(&ledger_dir, 10, None, None).expect("re-init repo");

    assert_eq!(
        repaired
            .get_repo_info()
            .expect("local info")
            .expect("present")
            .name,
        "default"
    );
    assert!(
        repaired
            .list_shadows_on_disk()
            .expect("list shadows after init")
            .contains(&peer_id)
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
    let selector = main
        .find_remote_repo_selector_by_id(&peer_id, wiki_info.uuid)
        .expect("resolve repaired shadow selector")
        .expect("shadow selector");
    assert!(
        main.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", selector))
            .exists()
    );
}

#[test]
fn remote_catalog_repair_does_not_borrow_local_metadata_for_shadow_naming() {
    let dir = TempDir::new().expect("create tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 10, Some("main"), Some("urn:main")).expect("main");
    let wiki = RepoManager::init(&ledger_dir, 10, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let peer_id = PeerId::new("peer-remote");
    let wiki_info = wiki.get_repo_info().expect("wiki info").expect("present");

    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;
    let poisoned = deve_core::ledger::RepoInfo {
        uuid: wiki_info.uuid,
        name: String::new(),
        url: wiki_info.url.clone(),
    };
    let write = wiki_db.begin_write().expect("write txn");
    write
        .open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(
            &0,
            bincode::serialize(&poisoned)
                .expect("serialize poisoned metadata")
                .as_slice(),
        )
        .expect("write poisoned metadata");
    write.commit().expect("commit metadata");

    main.ensure_shadow_db(&peer_id, &wiki_info.uuid)
        .expect("create legacy uuid shadow");

    let selectors = main
        .list_repos(Some(&peer_id))
        .expect("list repaired shadows");
    assert_eq!(selectors, vec![wiki_info.uuid.to_string()]);
    let selector = selectors[0].clone();
    assert!(
        main.remotes_dir()
            .join(peer_id.to_filename())
            .join(format!("{}.redb", selector))
            .exists()
    );
}
