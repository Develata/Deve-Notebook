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
