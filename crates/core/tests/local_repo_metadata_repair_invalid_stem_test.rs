#[cfg(unix)]
use deve_core::ledger::RepoManager;
#[cfg(unix)]
use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn repair_local_repo_catalog_fails_closed_on_invalid_repo_stem() {
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

    let err = repo
        .repair_local_repo_catalog()
        .expect_err("invalid repo stem must fail repair");
    assert!(err.to_string().contains("repairing local catalog"));
}
