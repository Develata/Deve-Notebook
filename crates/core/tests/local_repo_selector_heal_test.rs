use deve_core::ledger::RepoManager;
use tempfile::TempDir;

#[test]
fn resolve_local_repo_name_rejects_selector_mismatch_after_repair() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))
        .expect("init default");
    RepoManager::init(&ledger_dir, 8, Some("test"), Some("urn:test")).expect("init test");

    let default_id = repo
        .get_repo_info()
        .expect("default info")
        .expect("default present")
        .uuid;

    let err = repo
        .resolve_local_repo_name(Some(default_id), Some("test"))
        .expect_err("mismatched selector must fail");
    assert!(err.to_string().contains("Repo selector mismatch"));
}

#[test]
fn resolve_local_repo_name_for_execution_prefers_uuid_over_stale_name() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("default"), Some("urn:default"))
        .expect("init default");
    RepoManager::init(&ledger_dir, 8, Some("test"), Some("urn:test")).expect("init test");

    let default_id = repo
        .get_repo_info()
        .expect("default info")
        .expect("default present")
        .uuid;

    assert_eq!(
        repo.resolve_local_repo_name_for_execution(Some(default_id), Some("test"))
            .expect("uuid-first execution selector"),
        "default"
    );
}
