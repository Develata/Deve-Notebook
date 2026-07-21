use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

mod common;

#[test]
fn normal_startup_fails_closed_when_a_cataloged_authority_dir_is_missing() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) =
        common::init_cataloged_repo(&ledger_dir, &dir.path().join("notes")).expect("main");
    drop(repo);

    std::fs::remove_dir_all(ledger_dir.join("local")).expect("remove local catalog dir");

    let catalog_ids = deve_core::ledger::normal_catalog_ids_for_ledger(&ledger_dir)
        .expect("durable catalog remains readable");
    assert_eq!(catalog_ids, vec![repo_id]);
    let open_err = RepoManager::init_existing_for_repo_id(&ledger_dir, 8, repo_id)
        .err()
        .expect("missing cataloged authority must fail normal startup");
    assert!(
        open_err.to_string().contains("Local repo not found")
            || open_err
                .to_string()
                .contains("local authority database is missing"),
        "unexpected startup error: {open_err:#}"
    );
    assert!(
        !ledger_dir.join("local").exists(),
        "startup must not recreate authority"
    );
}

#[cfg(unix)]
#[test]
fn local_repo_catalog_calls_fail_closed_when_local_dir_is_unstatable() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, _repo_id) =
        common::init_cataloged_repo(&ledger_dir, &dir.path().join("notes")).expect("main");
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
    let (repo, repo_id) =
        common::init_cataloged_repo(&ledger_dir, &dir.path().join("notes")).expect("main");
    common::seed_non_file_local_repo_entry(&ledger_dir, "broken");

    assert_eq!(
        repo.list_repos(None).expect("normal listing"),
        vec![repo_id.to_string()]
    );
    assert_eq!(
        repo.list_local_repo_names_for_execution()
            .expect("normal execution listing"),
        vec![repo_id.to_string()]
    );
    assert_eq!(
        repo.find_local_repo_name_by_id(uuid::Uuid::new_v4())
            .expect("normal lookup ignores orphan debris"),
        None
    );
    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("non-file .redb entry must fail repair");

    assert!(repair_err.to_string().contains("expected file"));
}

#[test]
fn initial_bootstrap_fails_closed_when_local_catalog_path_is_file() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    std::fs::create_dir_all(&ledger_dir).expect("ledger dir");
    std::fs::write(ledger_dir.join("local"), b"not-a-directory").expect("poison local path");

    let error = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main"))
        .err()
        .expect("file local path must fail bootstrap");
    assert!(
        error.to_string().contains("无法创建本地库目录")
            || error.to_string().contains("file already exists"),
        "unexpected bootstrap error: {error:#}"
    );
}
