use deve_core::ledger::{REPO_METADATA, RepoInfo, RepoManager};
use deve_core::ledger::listing::RepoListing;
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
fn local_repo_listing_fails_closed_on_broken_secondary_repo() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    std::fs::write(local_dir.join("broken.redb"), b"not-a-redb").expect("broken local repo");

    let list_err = repo
        .list_repos(None)
        .expect_err("broken local repo must fail closed");
    assert!(list_err.to_string().contains("Broken local repo broken"));
    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("broken local repo must fail execution listing");
    assert!(exec_err.to_string().contains("Broken local repo broken"));
    assert_eq!(
        repo.resolve_local_repo_name(None, Some("main"))
            .expect("exact main selector remains valid"),
        "main"
    );
    let resolve_err = repo
        .resolve_local_repo_name(None, Some("broken"))
        .expect_err("broken selector must fail resolution");
    assert!(resolve_err.to_string().contains("Broken local repo broken"));
}

#[test]
fn init_fails_closed_on_broken_secondary_repo() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let _repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    std::fs::write(local_dir.join("broken.redb"), b"not-a-redb").expect("broken local repo");

    let err = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main"))
        .err()
        .expect("broken local repo must fail init");
    let detail = format!("{err:#}");
    assert!(detail.contains("Broken local repo broken"));
}

#[test]
fn set_vault_root_checked_fails_closed_on_broken_secondary_repo() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    std::fs::write(local_dir.join("broken.redb"), b"not-a-redb").expect("broken local repo");
    let vault_dir = dir.path().join("vault");
    std::fs::create_dir_all(&vault_dir).expect("vault dir");

    let err = repo
        .set_vault_root_checked(&vault_dir)
        .expect_err("broken local repo must fail checked vault mount");
    assert!(err.to_string().contains("Broken local repo broken"));
}

#[cfg(unix)]
#[test]
fn local_repo_listing_fails_closed_on_invalid_repo_stem() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    let invalid_path = {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        local_dir.join(OsString::from_vec(vec![0xff, b'.', b'r', b'e', b'd', b'b']))
    };
    let invalid = redb::Database::create(&invalid_path).expect("invalid stem db");
    invalid
        .begin_write()
        .expect("write txn")
        .commit()
        .expect("commit");
    drop(invalid);

    let list_err = repo
        .list_repos(None)
        .expect_err("invalid repo stem must fail local listing");
    assert!(list_err.to_string().contains("invalid file stem"));

    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("invalid repo stem must fail execution listing");
    assert!(exec_err.to_string().contains("invalid file stem"));

    let lookup_err = repo
        .find_local_repo_name_by_id(uuid::Uuid::new_v4())
        .expect_err("invalid repo stem must fail UUID lookup");
    assert!(lookup_err.to_string().contains("invalid file stem"));
}

#[test]
fn runtime_listing_fails_closed_on_duplicate_secondary_uuid_until_explicit_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let main_info = main.get_repo_info().expect("main info").expect("present");
    let wiki_db = wiki.open_database(None, "wiki").expect("wiki db");
    write_info(
        wiki_db.db.as_ref(),
        &RepoInfo {
            uuid: main_info.uuid,
            name: "wiki".into(),
            url: Some("urn:wiki".into()),
        },
    );

    let err = main
        .list_repos(None)
        .expect_err("runtime listing must fail closed on duplicate UUID");
    assert!(err.to_string().contains("duplicate local repository UUID"));

    main.repair_local_repo_catalog()
        .expect("explicit repair may rewrite duplicate UUID");
    assert_eq!(
        main.list_repos(None).expect("listing after repair"),
        vec!["main".to_string(), "wiki".to_string()]
    );
}

#[test]
fn runtime_listing_fails_closed_on_missing_secondary_metadata_until_explicit_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    let db = redb::Database::create(local_dir.join("legacy.redb")).expect("create legacy db");
    db.begin_write().expect("write txn").commit().expect("commit");
    drop(db);

    let err = repo
        .list_repos(None)
        .expect_err("runtime listing must fail closed on missing metadata");
    assert!(err.to_string().contains("repository metadata missing"));

    repo.repair_local_repo_catalog()
        .expect("explicit repair may bootstrap missing metadata");
    assert_eq!(
        repo.list_repos(None).expect("listing after repair"),
        vec!["legacy".to_string(), "main".to_string()]
    );
}
