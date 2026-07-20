use crate::ledger::RepoInfo;
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
