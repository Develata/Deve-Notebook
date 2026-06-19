use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use tempfile::TempDir;

mod common;

#[test]
fn init_repairs_duplicate_local_repo_uuid_and_name_drift() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    common::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let main_info = main.get_repo_info().expect("main info").expect("present");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;

    let bad = RepoInfo {
        uuid: main_info.uuid,
        name: "main".into(),
        url: Some(format!("urn:uuid:{}", main_info.uuid)),
    };
    common::write_repo_metadata(wiki_db.as_ref(), &bad);
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
    let second_info =
        common::create_initialized_local_repo(&ledger_dir, "wiki-1", "urn:test:wiki-b");
    let second_db = main.open_database(None, "wiki-1").expect("wiki-1 db").db;
    common::write_repo_metadata(
        second_db.as_ref(),
        &RepoInfo {
            uuid: second_info.uuid,
            name: "wiki".into(),
            url: Some("urn:test:wiki-b".into()),
        },
    );
    drop(second_db);

    let err = main
        .list_repos(None)
        .expect_err("duplicate local name drift must fail closed");
    assert!(err.to_string().contains("metadata name drifted to wiki"));

    main.repair_local_repo_catalog()
        .expect("repair local catalog");

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
    common::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");

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
    let notes_info = common::create_initialized_local_repo(&ledger_dir, "notes", "urn:notes");
    let other_db = main.open_database(None, "notes").expect("notes db").db;
    common::write_repo_metadata(
        other_db.as_ref(),
        &RepoInfo {
            uuid: notes_info.uuid,
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
fn init_allocates_collision_safe_repo_name_for_same_name_different_url() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let first = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("https://a.example"))
        .expect("init first wiki");
    let second = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("https://b.example"))
        .expect("init second wiki");

    let first_info = first.get_repo_info().expect("first info").expect("present");
    let second_info = second
        .get_repo_info()
        .expect("second info")
        .expect("present");
    assert_eq!(first_info.name, "wiki");
    assert_eq!(second_info.name, "wiki-1");
    assert_eq!(first_info.url.as_deref(), Some("https://a.example"));
    assert_eq!(second_info.url.as_deref(), Some("https://b.example"));
    assert_ne!(first_info.uuid, second_info.uuid);
    assert!(ledger_dir.join("local/wiki.redb").exists());
    assert!(ledger_dir.join("local/wiki-1.redb").exists());
}

#[test]
fn init_fails_closed_on_existing_local_repo_without_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    common::seed_metadata_less_local_repo(&ledger_dir, "legacy");

    let err = match RepoManager::init(&ledger_dir, 8, Some("legacy"), None) {
        Ok(_) => panic!("missing repo metadata must fail closed"),
        Err(err) => err,
    };
    assert!(err
        .to_string()
        .contains("repository metadata missing in existing database"));
}

#[test]
fn local_execution_resolution_ignores_broken_remote_catalogs() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let peer_id = deve_core::models::PeerId::new("peer-a");
    common::seed_broken_remote_shadow_repo(&ledger_dir, &peer_id, "broken");

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
