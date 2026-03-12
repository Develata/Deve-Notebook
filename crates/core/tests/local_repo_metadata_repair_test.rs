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
fn local_repo_listing_uses_collision_safe_labels_for_duplicate_names() {
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

    let repos = main.list_repos(None).expect("list repos");

    assert_eq!(repos.len(), 2);
    assert!(repos.contains(&"wiki".to_string()));
    assert!(repos.iter().any(|name| name.starts_with("wiki-")));
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
