use super::clear;
use crate::ledger::schema::PENDING_FS_OPS;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn clear_fails_closed_on_broken_pending_entry() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, repo_id) = crate::test_support::init_cataloged_repo(&ledger, &projection_base)?;
    let repo_name = repo_id.to_string();
    let repo = Arc::new(repo);

    repo.run_on_local_repo(&repo_name, |db| {
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(PENDING_FS_OPS)?;
            table.insert("notes/a.md", b"{broken-json".as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    })?;

    let err = clear(&repo, &repo_name, "notes/a.md")
        .expect_err("broken pending entry must fail closed during clear");
    assert!(
        !err.to_string().trim().is_empty(),
        "broken pending clear should surface a concrete error"
    );
    Ok(())
}
