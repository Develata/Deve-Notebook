use super::database_cache::evict_database_paths_under;
use super::node_check::check_node_consistency;
use super::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use super::{RepoManager, node_meta};
use crate::models::DocId;
use tempfile::TempDir;

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

#[test]
fn init_fails_closed_on_missing_nodes() -> anyhow::Result<()> {
    let _guard = crate::test_support::local_repo_catalog_test_guard();
    let tmp = TempDir::new()?;
    let ledger_dir = tmp.path().join("ledger");
    let (repo, repo_id) = crate::test_support::init_cataloged_repo_with_depth(
        &ledger_dir,
        &tmp.path().join("notes"),
        2,
    )?;
    let path = "notes/init-drift.md";
    let doc_id = seed_metadata_only_doc(&repo, path)?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        node_meta::remove_node_by_path(db, path)?;
        Ok(())
    })?;
    let report = repo.run_on_local_repo(repo.local_repo_name(), check_node_consistency)?;
    assert_eq!(report.missing_nodes, vec![(doc_id, path.to_string())]);
    drop(repo);
    evict_database_paths_under(&ledger_dir.join("local"))?;

    let err = match RepoManager::init_existing_for_repo_id(&ledger_dir, 2, repo_id) {
        Ok(_) => panic!("node drift must fail init closed"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("Node consistency drift detected"));
    Ok(())
}
