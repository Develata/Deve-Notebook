use super::RepoManager;
use crate::ledger::RepoInfo;

#[test]
fn set_vault_root_restores_previous_root_when_catalog_refresh_fails() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let dir = tempfile::tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    let first_vault = dir.path().join("vault-a");
    std::fs::create_dir_all(&first_vault)?;
    repo.set_vault_root(&first_vault);
    assert_eq!(repo.vault_root.as_deref(), Some(first_vault.as_path()));

    std::fs::remove_dir_all(dir.path().join("local"))?;

    let second_vault = dir.path().join("vault-b");
    repo.set_vault_root(&second_vault);

    assert_eq!(repo.vault_root.as_deref(), Some(first_vault.as_path()));
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

    let txn = wiki_db.begin_write()?;
    txn.delete_table(crate::ledger::REPO_METADATA)?;
    txn.commit()?;

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

    let txn = wiki_db.begin_write()?;
    txn.open_table(crate::ledger::REPO_METADATA)?.insert(
        &0,
        bincode::serialize(&crate::ledger::RepoInfo {
            uuid: wiki_info.uuid,
            name: "legacy-wiki".into(),
            url: wiki_info.url.clone(),
        })?
        .as_slice(),
    )?;
    txn.commit()?;

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

    let txn = main_db.begin_write()?;
    txn.open_table(crate::ledger::REPO_METADATA)?.insert(
        &0,
        bincode::serialize(&crate::ledger::RepoInfo {
            uuid: info.uuid,
            name: "legacy-main".into(),
            url: info.url.clone(),
        })?
        .as_slice(),
    )?;
    txn.commit()?;

    let err = main
        .resolve_local_repo_stem("legacy-main")
        .expect_err("stale main alias must fail selector resolution");
    assert!(
        err.to_string()
            .contains("metadata name drifted to legacy-main")
    );
    Ok(())
}
