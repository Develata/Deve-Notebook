use super::quarantine_md_dirs;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::PENDING_FS_OPS;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn quarantine_md_dirs_rolls_back_workspace_on_pending_cleanup_failure() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    let vault = dir.path().join("vault");
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);

    let md_dir = repo.local_repo_workspace_root("default")?.join("bad.md");
    std::fs::create_dir_all(&md_dir)?;
    std::fs::write(md_dir.join("note.txt"), "broken")?;

    repo.run_on_local_repo("default", |db| {
        let write = db.begin_write()?;
        {
            let mut table = write.open_table(PENDING_FS_OPS)?;
            table.insert("bad.md/file.md", b"{not-json".as_slice())?;
        }
        write.commit()?;
        Ok(())
    })?;

    let err = quarantine_md_dirs(&repo, &[String::from("default")])
        .expect_err("corrupt pending subtree must fail closed");
    assert!(md_dir.exists());
    assert!(
        !repo
            .local_repo_notegit_root("default")?
            .join("legacy-md-dir")
            .join("bad.md")
            .exists()
    );
    assert!(!err.to_string().is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn quarantine_md_dirs_fails_closed_on_unreadable_quarantine_target() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    let vault = dir.path().join("vault");
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);

    let md_dir = repo.local_repo_workspace_root("default")?.join("bad.md");
    std::fs::create_dir_all(&md_dir)?;
    std::fs::write(md_dir.join("note.txt"), "broken")?;

    let quarantine_root = repo
        .local_repo_notegit_root("default")?
        .join("legacy-md-dir");
    std::fs::create_dir_all(&quarantine_root)?;
    let original = std::fs::metadata(&quarantine_root)?.permissions();
    let mut blocked = original.clone();
    blocked.set_mode(0o000);
    std::fs::set_permissions(&quarantine_root, blocked)?;

    let err = quarantine_md_dirs(&repo, &[String::from("default")])
        .expect_err("unreadable quarantine target must fail closed");

    std::fs::set_permissions(&quarantine_root, original)?;
    assert!(md_dir.exists());
    assert!(
        err.to_string().contains("quarantine target path")
            || err.to_string().contains("Permission denied")
    );
    Ok(())
}
