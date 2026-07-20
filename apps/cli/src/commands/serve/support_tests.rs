use super::support::{ensure_native_loopback_default_workspace, init_runtime};

fn redb_names(ledger_dir: &std::path::Path) -> Vec<std::ffi::OsString> {
    let mut names = std::fs::read_dir(ledger_dir.join("local"))
        .expect("local repo directory")
        .map(|entry| entry.expect("local repo entry"))
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "redb"))
        .map(|entry| entry.file_name())
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn native_loopback_bootstrap_creates_default_workspace_for_empty_data_root() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");

    ensure_native_loopback_default_workspace(&ledger_dir, 8).expect("native bootstrap");

    let repo = init_runtime(&ledger_dir, 8).expect("repo");
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

    let repo = init_runtime(&ledger_dir, 8).expect("repo");
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

#[test]
fn serve_runtime_opens_exact_cataloged_repo_without_creating_an_orphan() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
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
    let before = redb_names(&ledger_dir);

    let repo = init_runtime(&ledger_dir, 8).expect("serve runtime");

    let after = redb_names(&ledger_dir);
    assert_eq!(after, before, "serve startup must not create a local Redb");
    assert_eq!(after.len(), 1, "fixture must contain one cataloged repo");
    let summaries = repo
        .list_cataloged_local_repo_summaries()
        .expect("catalog summaries");
    assert_eq!(summaries.len(), 1);
    assert_eq!(repo.local_repo_name(), summaries[0].repo_id.to_string());
}

#[test]
fn serve_runtime_rejects_an_empty_catalog_without_creating_local_repo_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    std::fs::create_dir_all(&ledger_dir).expect("empty ledger root");

    let error = init_runtime(&ledger_dir, 8)
        .err()
        .expect("empty catalog must fail closed");

    assert!(
        error.to_string().contains("run `deve init` first"),
        "{error}"
    );
    assert!(
        !ledger_dir.join("local").exists(),
        "failed serve startup must not create local authority storage"
    );
}
