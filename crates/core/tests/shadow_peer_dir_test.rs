use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn invalid_shadow_peer_dir_name_fails_closed_for_listing_and_repair() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let invalid_peer = repo
        .remotes_dir()
        .join(OsString::from_vec(vec![0xff, b'p', b'e', b'e', b'r']));
    std::fs::create_dir_all(&invalid_peer).expect("create invalid peer dir");

    let list_err = repo
        .list_shadows_on_disk()
        .expect_err("invalid peer dir must fail closed");
    assert!(list_err.to_string().contains("invalid directory name"));

    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("invalid peer dir must fail repair");
    assert!(repair_err.to_string().contains("invalid directory name"));
}

#[test]
fn non_directory_shadow_peer_entry_fails_closed_for_listing_and_repair() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    let invalid_peer = repo.remotes_dir().join("peer-file");
    std::fs::write(&invalid_peer, b"not-a-directory").expect("create invalid peer file");

    let list_err = repo
        .list_shadows_on_disk()
        .expect_err("non-directory peer entry must fail closed");
    assert!(list_err.to_string().contains("expected directory"));

    let repair_err = repo
        .repair_remote_repo_catalogs()
        .expect_err("non-directory peer entry must fail repair");
    assert!(repair_err.to_string().contains("expected directory"));
}
