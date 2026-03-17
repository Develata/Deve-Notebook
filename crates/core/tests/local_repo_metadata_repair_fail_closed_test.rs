use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use tempfile::TempDir;

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
