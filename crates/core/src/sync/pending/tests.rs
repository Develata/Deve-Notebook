use super::clear;
use crate::ledger::RepoManager;
use crate::ledger::schema::PENDING_FS_OPS;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn clear_fails_closed_on_broken_pending_entry() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos(&vault);
    let repo = Arc::new(repo);

    repo.run_on_local_repo("default", |db| {
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(PENDING_FS_OPS)?;
            table.insert("notes/a.md", b"{broken-json".as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    })?;

    let err = clear(&repo, "default", "notes/a.md")
        .expect_err("broken pending entry must fail closed during clear");
    assert!(
        !err.to_string().trim().is_empty(),
        "broken pending clear should surface a concrete error"
    );
    Ok(())
}
