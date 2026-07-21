use tempfile::TempDir;

mod common;

#[test]
fn resolve_local_repo_name_rejects_selector_mismatch_after_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, default_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("default-notes"), 8)
            .expect("init default");
    let test_id = common::add_cataloged_repo_with_depth(&repo, &dir.path().join("test-notes"), 8)
        .expect("init test");

    let err = repo
        .resolve_local_repo_name(Some(default_id), Some(&test_id.to_string()))
        .expect_err("mismatched selector must fail");
    assert!(err.to_string().contains("Repo selector mismatch"));
}

#[test]
fn resolve_local_repo_name_for_execution_rejects_selector_mismatch() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, default_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("default-notes"), 8)
            .expect("init default");
    let test_id = common::add_cataloged_repo_with_depth(&repo, &dir.path().join("test-notes"), 8)
        .expect("init test");

    let err = repo
        .resolve_local_repo_name_for_execution(Some(default_id), Some(&test_id.to_string()))
        .expect_err("mismatched execution selector must fail");
    assert!(err.to_string().contains("Repo selector mismatch"));
}
