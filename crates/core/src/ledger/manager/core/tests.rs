use crate::ledger::RepoInfo;
use crate::ledger::RepoManager;
use crate::test_support::init_cataloged_repo;

#[test]
fn resolve_local_selector_fails_closed_on_missing_secondary_metadata() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) = init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let (_wiki, wiki_id) = init_cataloged_repo(&ledger_dir, &dir.path().join("wiki-notes"))?;
    let wiki_db = main.open_database(None, &wiki_id.to_string())?.db;

    crate::test_support::delete_repo_metadata(wiki_db.as_ref())?;

    let err = main
        .resolve_local_repo_stem(&wiki_id.to_string())
        .expect_err("missing secondary metadata must fail selector resolution");
    assert!(err.to_string().contains("repository metadata missing"));
    Ok(())
}

#[test]
fn resolve_local_selector_fails_closed_on_stale_secondary_alias_drift() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) = init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let (wiki, wiki_id) = init_cataloged_repo(&ledger_dir, &dir.path().join("wiki-notes"))?;
    let wiki_info = wiki.get_repo_info()?.expect("wiki info");
    let wiki_db = main.open_database(None, &wiki_id.to_string())?.db;

    crate::test_support::write_repo_metadata(
        wiki_db.as_ref(),
        &RepoInfo {
            uuid: wiki_id,
            name: "legacy-wiki".into(),
            url: wiki_info.url.clone(),
        },
    )?;

    let err = main
        .resolve_local_repo_stem("legacy-wiki")
        .expect_err("stale secondary alias must fail selector resolution");
    assert!(
        err.to_string()
            .contains("metadata name drifted to legacy-wiki")
    );
    Ok(())
}

#[test]
fn resolve_local_selector_fails_closed_on_stale_main_alias_drift() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let (main, main_id) = init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let info = main.get_repo_info()?.expect("main info");
    let main_db = main.open_database(None, &main_id.to_string())?.db;

    crate::test_support::write_repo_metadata(
        main_db.as_ref(),
        &RepoInfo {
            uuid: main_id,
            name: "legacy-main".into(),
            url: info.url.clone(),
        },
    )?;

    let err = main
        .resolve_local_repo_stem("legacy-main")
        .expect_err("stale main alias must fail selector resolution");
    assert!(
        err.to_string()
            .contains("metadata name drifted to legacy-main")
    );
    Ok(())
}

#[test]
fn exact_existing_open_ignores_an_unrelated_corrupt_redb() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) = init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    std::fs::write(
        ledger_dir
            .join("local")
            .join(format!("{}.redb", uuid::Uuid::new_v4())),
        b"not a redb database",
    )?;

    let reopened = RepoManager::init_existing_for_repo_id(&ledger_dir, 8, repo_id)?;

    assert_eq!(reopened.local_repo_name(), repo.local_repo_name());
    Ok(())
}

#[test]
fn exact_existing_open_never_recreates_a_missing_database() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) = init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let db_path = ledger_dir.join("local").join(format!("{repo_id}.redb"));
    drop(repo);
    crate::ledger::database_cache::evict_database_paths_under(&ledger_dir.join("local"))?;
    std::fs::remove_file(&db_path)?;

    let error = RepoManager::init_existing_for_repo_id(&ledger_dir, 8, repo_id)
        .err()
        .expect("missing exact authority database must fail closed");

    assert!(error.to_string().contains("Local repo not found for UUID"));
    assert!(
        !db_path.exists(),
        "open-existing must not recreate authority"
    );
    Ok(())
}
