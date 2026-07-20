use deve_core::ledger::node_check::check_node_consistency;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::ledger::{RepoManager, node_meta};
use deve_core::models::DocId;
use tempfile::TempDir;

mod common;

#[test]
fn node_check_detects_missing_nodes() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let ledger_dir = tmp.path().join("ledger");
    let (repo, _repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &tmp.path().join("notes"), 2)?;
    let path = "notes/init-drift.md";
    let doc_id = seed_metadata_only_doc(&repo, path)?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        node_meta::remove_node_by_path(db, path)?;
        Ok(())
    })?;
    let report = repo.run_on_local_repo(repo.local_repo_name(), check_node_consistency)?;
    assert_eq!(report.missing_nodes, vec![(doc_id, path.to_string())]);
    Ok(())
}

fn seed_metadata_only_doc(repo: &RepoManager, path: &str) -> anyhow::Result<DocId> {
    let doc_id = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.insert(path, doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), path)?;
        }
        write_txn.commit()?;
        Ok(())
    })?;
    Ok(doc_id)
}
