use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use tempfile::TempDir;

fn write_info(db: &redb::Database, info: &RepoInfo) {
    let txn = db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA)
        .expect("repo metadata")
        .insert(&0, bincode::serialize(info).expect("serialize").as_slice())
        .expect("write metadata");
    txn.commit().expect("commit");
}

#[test]
fn init_repairs_duplicate_local_repo_uuid_and_name_drift() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let main_info = main.get_repo_info().expect("main info").expect("present");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;

    let bad = RepoInfo {
        uuid: main_info.uuid,
        name: "main".into(),
        url: Some(format!("urn:uuid:{}", main_info.uuid)),
    };
    write_info(wiki_db.as_ref(), &bad);
    main.repair_local_repo_catalog().expect("repair catalog");

    let repaired_main = main
        .get_repo_info()
        .expect("repaired main info")
        .expect("main present");
    let repaired_wiki = main
        .get_repo_info_for(None, Some("wiki"))
        .expect("wiki lookup")
        .expect("wiki present");

    assert_eq!(repaired_main.name, "main");
    assert_eq!(repaired_wiki.name, "wiki");
    assert_ne!(repaired_wiki.uuid, repaired_main.uuid);
    assert_eq!(
        repaired_wiki.url.as_deref(),
        Some(format!("urn:uuid:{}", repaired_wiki.uuid).as_str())
    );
    assert_eq!(
        main.list_repos(None).expect("list local repos"),
        vec!["main".to_string(), "wiki".to_string()]
    );
}

#[test]
fn local_repo_listing_fails_closed_on_duplicate_name_drift_until_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main =
        RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:test:wiki-a")).expect("main");
    let local_dir = ledger_dir.join("local");
    let second_path = local_dir.join("wiki-1.redb");
    let second_db = redb::Database::create(&second_path).expect("create second db");
    let txn = second_db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA).expect("repo metadata");
    txn.commit().expect("commit metadata table");
    write_info(
        &second_db,
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "wiki".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    );
    drop(second_db);

    let err = main
        .list_repos(None)
        .expect_err("duplicate local name drift must fail closed");
    assert!(err.to_string().contains("metadata name drifted to wiki"));

    main.repair_local_repo_catalog().expect("repair local catalog");

    let repos = main.list_repos(None).expect("list repos after repair");
    assert_eq!(repos.len(), 2);
    assert!(repos.contains(&"wiki".to_string()));
    assert!(repos.contains(&"wiki-1".to_string()));
}

#[test]
fn local_repo_execution_requires_explicit_selector_when_multiple_repos_exist() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");

    let err = main
        .resolve_local_repo_name_for_execution(None, None)
        .expect_err("multiple repos must require explicit selector");

    assert!(err.to_string().contains("multiple local repos exist"));
}

#[test]
fn repair_rewrites_duplicate_local_repo_url_to_repo_urn() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let other_path = ledger_dir.join("local").join("notes.redb");
    let other_db = redb::Database::create(&other_path).expect("create notes db");
    let txn = other_db.begin_write().expect("write txn");
    txn.open_table(REPO_METADATA).expect("repo metadata");
    txn.commit().expect("commit metadata table");
    write_info(
        &other_db,
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "notes".into(),
            url: Some("urn:main".into()),
        },
    );
    drop(other_db);

    main.repair_local_repo_catalog().expect("repair catalog");

    let repaired = main
        .get_repo_info_for(None, Some("notes"))
        .expect("lookup notes")
        .expect("notes present");
    assert_eq!(
        repaired.url.as_deref(),
        Some(format!("urn:uuid:{}", repaired.uuid).as_str())
    );
}

#[test]
fn init_without_url_does_not_reuse_same_name_repo_with_explicit_url() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let first = RepoManager::init(
        &ledger_dir,
        8,
        Some("wiki"),
        Some("https://example.com/wiki.git"),
    )
    .expect("init explicit wiki");
    let second = RepoManager::init(&ledger_dir, 8, Some("wiki"), None).expect("init implicit wiki");

    let first_info = first.get_repo_info().expect("first info").expect("present");
    let second_info = second
        .get_repo_info()
        .expect("second info")
        .expect("present");
    assert_eq!(first_info.name, "wiki");
    assert_eq!(second_info.name, "wiki-1");
    assert_ne!(first_info.uuid, second_info.uuid);
}

#[test]
fn init_repairs_existing_local_repo_without_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir).expect("create local dir");
    let db_path = local_dir.join("legacy.redb");
    let db = redb::Database::create(&db_path).expect("create legacy db");
    let txn = db.begin_write().expect("write txn");
    txn.commit().expect("commit empty db");
    drop(db);

    let repo = RepoManager::init(&ledger_dir, 8, Some("legacy"), None).expect("init legacy repo");
    let info = repo.get_repo_info().expect("repo info").expect("present");

    assert_eq!(info.name, "legacy");
    assert_eq!(
        info.url.as_deref(),
        Some(format!("urn:uuid:{}", info.uuid).as_str())
    );
    assert_eq!(
        repo.list_repos(None).expect("list repos"),
        vec!["legacy".to_string()]
    );
}

#[test]
fn local_execution_resolution_ignores_broken_remote_catalogs() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let remote_dir = ledger_dir.join("remotes").join("peer-a");
    std::fs::create_dir_all(&remote_dir).expect("remote dir");
    std::fs::write(remote_dir.join("broken.redb"), b"not-a-redb").expect("broken shadow file");

    assert_eq!(
        repo.resolve_local_repo_name_for_execution(None, Some("main"))
            .expect("local execution selector"),
        "main"
    );
    assert_eq!(
        repo.list_local_repo_names_for_execution()
            .expect("local repo names"),
        vec!["main".to_string()]
    );
}
