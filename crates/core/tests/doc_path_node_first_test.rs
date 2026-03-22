use deve_core::ledger::RepoManager;
use deve_core::ledger::node_meta;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use tempfile::TempDir;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    (dir, repo)
}

#[test]
fn doc_path_lookup_prefers_node_projection_over_stale_metadata() {
    let (_dir, repo) = new_repo();
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), "notes/a.md", None, "test")
        .expect("create file structure");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.remove("notes/a.md")?;
            p2d.insert("stale/a.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "stale/a.md")?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("poison metadata path only");

    let meta = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            node_meta::file_meta_for_doc(db, doc_id)
        })
        .expect("load node meta")
        .expect("existing file meta");
    assert_eq!(meta.path, "notes/a.md");
}
