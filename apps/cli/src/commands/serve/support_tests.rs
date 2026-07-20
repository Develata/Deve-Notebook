use super::support::ensure_native_loopback_default_workspace;
use deve_core::ledger::RepoManager;

#[test]
fn native_loopback_bootstrap_creates_default_workspace_for_empty_data_root() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");

    ensure_native_loopback_default_workspace(&ledger_dir, 8).expect("native bootstrap");

    let repo = RepoManager::init(&ledger_dir, 8, None, None).expect("repo");
    repo.validate_projection_locator_map().expect("locator map");
    // Machine names are canonical UUID strings and resolution does not consult
    // the host-local "default" alias, so resolve the bootstrapped repo through
    // the durable catalog.
    let summaries = repo
        .list_cataloged_local_repo_summaries()
        .expect("catalog summaries");
    assert_eq!(
        summaries.len(),
        1,
        "bootstrap must catalog exactly one repo"
    );
    let workspace = repo
        .local_repo_workspace_root(&summaries[0].execution_name)
        .expect("default workspace");
    let projection_base = std::fs::canonicalize(dir.path().join("notes")).expect("canonical notes");
    assert!(workspace.starts_with(projection_base));
    assert!(workspace.join(".notegit/identity.toml").is_file());
    assert!(dir.path().join("ledger/.host/keys").is_dir());
}

#[test]
fn native_loopback_bootstrap_preserves_existing_valid_locator() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("custom-notes");
    crate::commands::init::run(
        &ledger_dir,
        "default",
        &projection_base,
        dir.path().to_path_buf(),
        8,
        None,
        None,
    )
    .expect("init");

    ensure_native_loopback_default_workspace(&ledger_dir, 8).expect("native bootstrap");

    let repo = RepoManager::init(&ledger_dir, 8, None, None).expect("repo");
    let summaries = repo
        .list_cataloged_local_repo_summaries()
        .expect("catalog summaries");
    assert_eq!(
        summaries.len(),
        1,
        "bootstrap must preserve the single cataloged repo"
    );
    let locator = repo
        .projection_locator_for_local_repo(&summaries[0].execution_name)
        .expect("locator");
    assert_eq!(
        locator.projection_base_abs,
        std::fs::canonicalize(&projection_base).expect("canonical custom notes")
    );
}
