use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[test]
fn local_repo_listing_fails_closed_when_local_catalog_dir_is_missing() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki")).expect("wiki");
    let wiki_id = wiki
        .get_repo_info()
        .expect("wiki info")
        .expect("present")
        .uuid;

    std::fs::remove_dir_all(ledger_dir.join("local")).expect("remove local catalog dir");

    let list_err = repo
        .list_repos(None)
        .expect_err("missing local repo dir must fail listing");
    assert!(
        list_err
            .to_string()
            .contains("local repo directory missing")
    );

    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("missing local repo dir must fail execution listing");
    assert!(
        exec_err
            .to_string()
            .contains("local repo directory missing")
    );

    let lookup_err = repo
        .find_local_repo_name_by_id(wiki_id)
        .expect_err("missing local repo dir must fail UUID lookup");
    assert!(
        lookup_err
            .to_string()
            .contains("local repo directory missing")
    );
}

#[cfg(unix)]
#[test]
fn local_repo_catalog_calls_fail_closed_when_local_dir_is_unstatable() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    let original = std::fs::metadata(&local_dir)
        .expect("metadata")
        .permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&local_dir, blocked).expect("chmod 000");

    let list_err = repo
        .list_repos(None)
        .expect_err("unstatable local dir must fail listing");
    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("unstatable local dir must fail execution listing");
    let lookup_err = repo
        .find_local_repo_name_by_id(uuid::Uuid::new_v4())
        .expect_err("unstatable local dir must fail UUID lookup");
    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("unstatable local dir must fail repair");

    std::fs::set_permissions(&local_dir, original).expect("restore perms");
    for err in [&list_err, &exec_err, &lookup_err, &repair_err] {
        let detail = err.to_string();
        assert!(
            detail.contains("Failed to stat local repo directory")
                || detail.contains("Permission denied"),
            "unexpected error detail: {detail}"
        );
    }
}

#[test]
fn local_repo_catalog_fails_closed_on_non_file_redb_entry() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    std::fs::create_dir_all(ledger_dir.join("local").join("broken.redb"))
        .expect("create fake repo dir");

    let list_err = repo
        .list_repos(None)
        .expect_err("non-file .redb entry must fail listing");
    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("non-file .redb entry must fail execution listing");
    let lookup_err = repo
        .find_local_repo_name_by_id(uuid::Uuid::new_v4())
        .expect_err("non-file .redb entry must fail UUID lookup");
    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("non-file .redb entry must fail repair");

    for err in [&list_err, &exec_err, &lookup_err, &repair_err] {
        assert!(err.to_string().contains("expected file"));
    }
}

#[test]
fn local_repo_catalog_calls_fail_closed_when_local_catalog_path_is_file() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    std::fs::remove_dir_all(ledger_dir.join("local")).expect("remove local dir");
    std::fs::write(ledger_dir.join("local"), b"not-a-directory").expect("poison local path");

    let list_err = repo
        .list_repos(None)
        .expect_err("file local path must fail listing");
    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("file local path must fail execution listing");
    let lookup_err = repo
        .find_local_repo_name_by_id(uuid::Uuid::new_v4())
        .expect_err("file local path must fail UUID lookup");
    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("file local path must fail repair");

    for err in [&list_err, &exec_err, &lookup_err, &repair_err] {
        assert!(err.to_string().contains("expected directory"));
    }
}
