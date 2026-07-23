use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

mod common;

#[test]
fn local_repo_listing_ignores_uncataloged_broken_artifact_and_repair_reports_it() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, _repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");
    common::seed_broken_local_repo_file(&ledger_dir, "broken");

    assert_eq!(
        repo.list_repos(None).expect("catalog listing"),
        vec![repo.local_repo_name().to_string()]
    );
    assert_eq!(
        repo.list_local_repo_names_for_execution()
            .expect("execution listing"),
        vec![repo.local_repo_name().to_string()]
    );
    assert_eq!(
        repo.resolve_local_repo_name(None, Some(repo.local_repo_name()))
            .expect("exact RepoId execution selector remains valid"),
        repo.local_repo_name()
    );
    let resolve_err = repo
        .resolve_local_repo_name(None, Some("broken"))
        .expect_err("uncataloged selector must fail resolution");
    assert!(resolve_err.to_string().contains("Local repo not found"));
    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("explicit repair must report broken orphan");
    assert!(
        repair_err
            .to_string()
            .contains("physical authority selector is not a RepoId: broken"),
        "{repair_err:#}"
    );
}

#[test]
fn init_ignores_uncataloged_broken_artifact_and_repair_reports_it() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");
    common::seed_broken_local_repo_file(&ledger_dir, "broken");
    drop(repo);

    let reopened = RepoManager::init(&ledger_dir, 8, None, None).expect("cataloged init");
    assert_eq!(
        reopened.list_repos(None).expect("catalog listing"),
        vec![repo_id.to_string()]
    );
    let err = reopened
        .repair_local_repo_catalog()
        .expect_err("explicit repair must report broken orphan");
    assert!(
        format!("{err:#}").contains("physical authority selector is not a RepoId: broken"),
        "{err:#}"
    );
}

#[cfg(unix)]
#[test]
fn init_fails_closed_on_unstatable_local_db_path() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, _repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");
    let local_dir = ledger_dir.join("local");
    let original = std::fs::metadata(&local_dir)
        .expect("metadata")
        .permissions();
    let mut perms = original.clone();
    perms.set_mode(0o000);
    drop(repo);
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
fn projection_mount_ignores_uncataloged_broken_artifact_and_repair_reports_it() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let first_projection_base = dir.path().join("notes-ok");
    let (mut repo, repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &first_projection_base, 8)
            .expect("main");
    common::seed_broken_local_repo_file(&ledger_dir, "broken");
    let projection_base = dir.path().join("notes");
    std::fs::create_dir_all(&projection_base).expect("projection base dir");

    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("cataloged projection mount");
    assert!(repo.local_repo_workspace_root(&repo_id.to_string()).is_ok());
    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("explicit repair must report broken orphan");
    assert!(
        repair_err
            .to_string()
            .contains("physical authority selector is not a RepoId: broken"),
        "{repair_err:#}"
    );
}

#[cfg(unix)]
#[test]
fn local_repo_listing_ignores_invalid_nonmember_stem_and_repair_reports_it() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");
    common::seed_invalid_stem_local_repo(&ledger_dir);

    assert_eq!(
        repo.list_repos(None).expect("catalog listing"),
        vec![repo_id.to_string()]
    );
    assert_eq!(
        repo.list_local_repo_names_for_execution()
            .expect("execution listing"),
        vec![repo_id.to_string()]
    );
    assert_eq!(
        repo.find_local_repo_name_by_id(uuid::Uuid::new_v4())
            .expect("catalog lookup"),
        None
    );
    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("explicit repair must report invalid file stem");
    assert!(repair_err.to_string().contains("invalid file stem"));
}

#[test]
fn runtime_and_explicit_repair_fail_closed_on_secondary_repo_id_mismatch() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (main, main_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("main-notes"), 8)
            .expect("main");
    let wiki_id = common::add_cataloged_repo_with_depth(&main, &dir.path().join("wiki-notes"), 8)
        .expect("wiki");
    let wiki_db = main
        .lease_local_authority(wiki_id)
        .expect("wiki authority lease");
    common::write_repo_metadata(
        wiki_db.db(),
        &RepoInfo {
            uuid: main_id,
            name: main_id.to_string(),
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
    assert_eq!(common::read_repo_metadata(wiki_db.db()).uuid, main_id);
}

#[test]
fn runtime_ignores_uncataloged_metadata_less_artifact_and_repair_reports_it() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)
            .expect("main");
    let missing_id = uuid::Uuid::new_v4();
    common::seed_metadata_less_local_repo(&ledger_dir, &missing_id.to_string());

    assert_eq!(
        repo.list_repos(None).expect("catalog listing"),
        vec![repo_id.to_string()]
    );

    let repair_err = repo
        .repair_local_repo_catalog()
        .expect_err("explicit repair must not invent repository identity");
    assert!(
        repair_err
            .to_string()
            .contains("repository metadata missing"),
        "unexpected repair error: {repair_err:#}"
    );
}

#[cfg(unix)]
#[test]
fn repair_local_repo_catalog_fails_closed_on_unstatable_workspace_root() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes-base");
    let (main, _main_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &projection_base, 8).expect("main");
    common::add_cataloged_repo_with_url(&main, &projection_base, "urn:wiki")
        .expect("initialized wiki");

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
