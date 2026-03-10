use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use tempfile::TempDir;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    (dir, repo)
}

#[test]
fn metadata_only_path_mapping_is_hidden_from_business_lookup() {
    let (_dir, repo) = new_repo();
    let doc_id = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("seed metadata-only mapping");

    assert_eq!(
        repo.get_docid("notes/legacy.md").expect("lookup by path"),
        None
    );
    assert_eq!(
        repo.get_path_by_docid(doc_id).expect("lookup by doc id"),
        None
    );
}
