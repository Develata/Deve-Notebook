use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

mod common;

#[test]
fn local_repo_listing_fails_closed_on_broken_secondary_repo() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    common::seed_broken_local_repo_file(&ledger_dir, "broken");

    let list_err = repo
        .list_repos(None)
        .expect_err("broken local repo must fail closed");
    assert!(list_err.to_string().contains("Broken local repo broken"));
    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("broken local repo must fail execution listing");
    assert!(exec_err.to_string().contains("Broken local repo broken"));
    assert_eq!(
        repo.resolve_local_repo_name(None, Some(repo.local_repo_name()))
            .expect("exact RepoId execution selector remains valid"),
        repo.local_repo_name()
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
    common::seed_broken_local_repo_file(&ledger_dir, "broken");

    let err = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main"))
        .err()
        .expect("broken local repo must fail init");
    let detail = format!("{err:#}");
    assert!(detail.contains("Broken local repo broken"));
}

#[cfg(unix)]
#[test]
fn init_fails_closed_on_unstatable_local_db_path() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let _repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    let original = std::fs::metadata(&local_dir)
        .expect("metadata")
        .permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    std::fs::set_permissions(&local_dir, perms).expect("chmod 000");

    let err = match RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")) {
        Ok(_) => panic!("unstatable local db path must fail init"),
        Err(err) => err,
    };

    std::fs::set_permissions(&local_dir, original).expect("restore perms");
    assert!(
        err.to_string()
            .contains("Failed to stat local database path during init")
            || err.to_string().contains("Permission denied")
    );
}

#[test]
fn set_projection_base_for_all_local_repos_checked_fails_closed_on_broken_secondary_repo() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let mut repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let first_projection_base = dir.path().join("notes-ok");
    std::fs::create_dir_all(&first_projection_base).expect("first projection base dir");
    repo.set_projection_base_for_all_local_repos_checked(&first_projection_base)
        .expect("initial projection base mount");
    common::seed_broken_local_repo_file(&ledger_dir, "broken");
    let projection_base = dir.path().join("notes");
    std::fs::create_dir_all(&projection_base).expect("projection base dir");

    let err = repo
        .set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect_err("broken local repo must fail checked projection base mount");
    assert!(err.to_string().contains("Broken local repo broken"));
    let root_err = repo
        .local_repo_workspace_root("main")
        .expect_err("workspace root lookup must also fail closed on broken catalog");
    assert!(root_err.to_string().contains("Broken local repo broken"));
}

#[cfg(unix)]
#[test]
fn local_repo_listing_fails_closed_on_invalid_repo_stem() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    common::seed_invalid_stem_local_repo(&ledger_dir);

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
fn runtime_and_explicit_repair_fail_closed_on_secondary_repo_id_mismatch() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    common::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let main_info = main.get_repo_info().expect("main info").expect("present");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db");
    common::write_repo_metadata(
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
    assert!(err.to_string().contains("physical RepoId does not match"));

    let repair_err = main
        .repair_local_repo_catalog()
        .expect_err("explicit repair must not rewrite duplicate UUID");
    assert!(
        repair_err
            .to_string()
            .contains("physical RepoId does not match")
    );
    assert_eq!(
        common::read_repo_metadata(wiki_db.db.as_ref()).uuid,
        main_info.uuid
    );
}

#[test]
fn runtime_and_explicit_repair_fail_closed_on_missing_secondary_metadata() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let missing_id = uuid::Uuid::new_v4();
    common::seed_metadata_less_local_repo(&ledger_dir, &missing_id.to_string());

    let err = repo
        .list_repos(None)
        .expect_err("runtime listing must fail closed on missing metadata");
    assert!(err.to_string().contains("repository metadata missing"));

    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("explicit repair must not invent repository identity");
    assert!(
        repair_err
            .to_string()
            .contains("repository metadata missing")
    );
}

#[cfg(unix)]
#[test]
fn repair_local_repo_catalog_fails_closed_on_unstatable_workspace_root() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let mut main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki_info = common::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let projection_base = dir.path().join("notes-base");
    std::fs::create_dir_all(projection_base.join("notes")).expect("workspace root");
    main.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("mount projection base");
    let wiki_db = main.open_database(None, "wiki").expect("wiki db").db;
    common::write_repo_metadata(
        wiki_db.as_ref(),
        &RepoInfo {
            uuid: wiki_info.uuid,
            name: "notes".into(),
            url: wiki_info.url.clone(),
        },
    );

    let original = std::fs::metadata(&projection_base)
        .expect("metadata")
        .permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&projection_base, blocked).expect("chmod 000");

    let err = main
        .repair_local_repo_catalog()
        .expect_err("unstatable workspace root must fail closed");

    std::fs::set_permissions(&projection_base, original).expect("restore perms");
    let err_text = err.to_string();
    assert!(
        err_text.contains("Failed to stat current workspace root while repairing local catalog")
            || err_text
                .contains("Failed to stat previous workspace root while repairing local catalog")
            || err_text.contains("Permission denied"),
        "{err_text}"
    );
}
