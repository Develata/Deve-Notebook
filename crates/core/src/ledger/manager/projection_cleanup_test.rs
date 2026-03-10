use super::drop_unanchored_projection_path;
use crate::ledger::RepoManager;
use crate::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use crate::models::DocId;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

#[test]
fn drops_legacy_projection_without_ledger_facts() {
    let (_dir, repo) = new_repo();
    let doc_id = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/new.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/new.md")?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("seed metadata-only mapping");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        drop_unanchored_projection_path(db, "notes/new.md")
    })
    .expect("drop transient projection");

    assert!(
        repo.get_docid("notes/new.md")
            .expect("lookup doc id")
            .is_none()
    );
}

#[test]
fn keeps_empty_file_projection_backed_by_structure_facts() {
    let (_dir, repo) = new_repo();
    let doc_id = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), "notes/empty.md", None, "test")
        .expect("create empty file structure");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        drop_unanchored_projection_path(db, "notes/empty.md")
    })
    .expect("drop transient projection");

    assert_eq!(
        repo.get_docid("notes/empty.md").expect("lookup doc id"),
        Some(doc_id)
    );
}
