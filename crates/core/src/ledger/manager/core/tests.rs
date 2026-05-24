use super::RepoManager;
use crate::ledger::RepoInfo;
use std::path::Path;

fn locator_base_from_file(path: &Path) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    Ok(value["locators"][0]["projection_base_abs"]
        .as_str()
        .expect("projection base")
        .to_string())
}

#[test]
fn set_projection_base_for_all_local_repos_restores_previous_root_when_catalog_refresh_fails()
-> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    let first_projection_base = dir.path().join("notes-a");
    std::fs::create_dir_all(&first_projection_base)?;
    repo.set_projection_base_for_all_local_repos(&first_projection_base);
    assert_eq!(
        repo.local_repo_workspace_root("default")?,
        std::fs::canonicalize(&first_projection_base)?.join("default")
    );

    std::fs::remove_dir_all(ledger.join("local"))?;

    let second_projection_base = dir.path().join("notes-b");
    repo.set_projection_base_for_all_local_repos(&second_projection_base);

    assert_eq!(
        locator_base_from_file(&repo.projection_locator_path())?,
        std::fs::canonicalize(&first_projection_base)?
            .to_string_lossy()
            .to_string()
    );
    Ok(())
}

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
