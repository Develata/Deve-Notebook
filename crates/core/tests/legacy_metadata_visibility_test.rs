use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init cataloged repo");
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
    assert!(
        repo.get_file_meta_for_doc(doc_id)
            .expect("lookup by doc id")
            .is_none()
    );
}
