use deve_core::ledger::node_check::check_node_consistency;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::ledger::{RepoManager, node_meta};
use deve_core::models::DocId;
use tempfile::TempDir;

#[test]
fn init_does_not_auto_repair_missing_nodes() -> anyhow::Result<()> {
    let tmp = TempDir::new()?;
    let ledger_dir = tmp.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, None, None)?;
    let path = "notes/init-drift.md";
    let doc_id = seed_metadata_only_doc(&repo, path)?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        node_meta::remove_node_by_path(db, path)?;
        Ok(())
    })?;
    drop(repo);

    let repo = RepoManager::init(&ledger_dir, 2, None, None)?;
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
