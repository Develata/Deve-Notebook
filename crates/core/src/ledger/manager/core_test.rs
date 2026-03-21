use super::RepoManager;

#[test]
fn set_vault_root_restores_previous_root_when_catalog_refresh_fails() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    let first_vault = dir.path().join("vault-a");
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
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main"))?;
    let wiki = RepoManager::init(&ledger_dir, 8, Some("wiki"), Some("urn:wiki"))?;
    let wiki_db = main.open_database(None, "wiki")?.db;

    let txn = wiki_db.begin_write()?;
    txn.delete_table(crate::ledger::REPO_METADATA)?;
    txn.commit()?;
    drop(wiki);

    let err = main
        .resolve_local_repo_stem("wiki")
        .expect_err("missing secondary metadata must fail selector resolution");
    assert!(err.to_string().contains("repository metadata missing"));
    Ok(())
}
