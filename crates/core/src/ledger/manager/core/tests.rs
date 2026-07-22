use crate::ledger::RepoInfo;
use crate::ledger::RepoManager;
use crate::test_support::init_cataloged_repo;

#[test]
fn empty_host_has_no_implicit_primary_and_never_creates_local_redb() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");

    let repo = RepoManager::init_empty_host(&ledger_dir, 8)?;

    assert!(repo.list_cataloged_local_repo_summaries()?.is_empty());
    assert!(repo.local_repo_name().is_empty());
    assert_eq!(
        repo.list_local_docs(None)
            .expect_err("NoScope default read must fail closed")
            .to_string(),
        "no local repository is selected"
    );
    assert!(std::fs::read_dir(ledger_dir.join("local"))?.all(|entry| {
        entry
            .map(|entry| entry.path().extension().is_none_or(|ext| ext != "redb"))
            .unwrap_or(false)
    }));
    Ok(())
}

#[test]
fn repo_manager_retains_one_canonical_absolute_ledger_root() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("parent").join("..").join("ledger");
    std::fs::create_dir_all(dir.path().join("parent"))?;

    let repo = RepoManager::init_empty_host(&ledger_dir, 8)?;

    assert!(repo.ledger_dir().is_absolute());
    assert_eq!(
        repo.ledger_dir(),
        std::fs::canonicalize(dir.path().join("ledger"))?
    );
    Ok(())
}

#[test]
fn resolve_local_selector_fails_closed_on_missing_secondary_metadata() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let (main, _main_id) = init_cataloged_repo(&ledger_dir, &dir.path().join("notes"))?;
    let wiki_id = crate::test_support::add_cataloged_repo(
        &main,
        &ledger_dir,
        &dir.path().join("wiki-notes"),
        None,
    )?;
    let wiki_db = main.local_authority_lease_for_test(wiki_id)?;

    crate::test_support::delete_repo_metadata(wiki_db.db())?;

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
    let wiki_id = crate::test_support::add_cataloged_repo(
        &main,
        &ledger_dir,
        &dir.path().join("wiki-notes"),
        None,
    )?;
    let wiki_info = main
        .repo_scope_runtime()
        .get_local_repo_info_by_id_without_repair(wiki_id)?
        .expect("wiki info");
    let wiki_db = main.local_authority_lease_for_test(wiki_id)?;

    crate::test_support::write_repo_metadata(
        wiki_db.db(),
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
    let main_db = main.local_authority_lease_for_test(main_id)?;

    crate::test_support::write_repo_metadata(
        main_db.db(),
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
    let expected_name = repo.local_repo_name().to_string();
    drop(repo);

    let reopened = RepoManager::init_existing_for_repo_id(&ledger_dir, 8, repo_id)?;

    assert_eq!(reopened.local_repo_name(), expected_name);
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

#[test]
fn exact_existing_open_never_admits_an_uncataloged_database() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let repo_id = uuid::Uuid::new_v4();
    let repo = crate::ledger::init::init_with_options(
        &ledger_dir,
        8,
        Some("prepared"),
        crate::ledger::init::RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: None,
        },
    )?;
    drop(repo);

    let error = RepoManager::init_existing_for_repo_id(&ledger_dir, 8, repo_id)
        .err()
        .expect("uncataloged authority database must require explicit repair");

    assert!(
        error
            .to_string()
            .contains("is not a durable Normal catalog member")
    );
    Ok(())
}
