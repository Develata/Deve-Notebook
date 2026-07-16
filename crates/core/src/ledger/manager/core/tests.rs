use super::RepoManager;
use crate::ledger::RepoInfo;

#[test]
fn resolve_local_selector_fails_closed_on_missing_secondary_metadata() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main"))?;
    crate::test_support::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let wiki_db = main.open_database(None, "wiki")?.db;

    crate::test_support::delete_repo_metadata(wiki_db.as_ref())?;

    let err = main
        .resolve_local_repo_stem("wiki")
        .expect_err("missing secondary metadata must fail selector resolution");
    assert!(err.to_string().contains("repository metadata missing"));
    Ok(())
}

#[test]
fn resolve_local_selector_fails_closed_on_stale_secondary_alias_drift() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main"))?;
    let wiki_info = RepoInfo {
        uuid: crate::test_support::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki")
            .uuid,
        name: "wiki".into(),
        url: Some("urn:wiki".into()),
    };
    let wiki_db = main.open_database(None, "wiki")?.db;

    crate::test_support::write_repo_metadata(
        wiki_db.as_ref(),
        &crate::ledger::RepoInfo {
            uuid: wiki_info.uuid,
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
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main"))?;
    let info = main.get_repo_info()?.expect("main info");
    let main_db = main.open_database(None, "main")?.db;

    crate::test_support::write_repo_metadata(
        main_db.as_ref(),
        &crate::ledger::RepoInfo {
            uuid: info.uuid,
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
