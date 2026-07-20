use super::*;
use crate::ledger::ops::{
    count_ops_from_db, get_ops_from_db, get_ops_from_db_after, get_structure_ops_for_node_from_db,
    max_seq_from_db,
};
use crate::ledger::schema::{DOC_OPS, LEDGER_OPS, NODE_OPS};
use crate::models::NodeId;
use anyhow::Result;
use tempfile::TempDir;

fn new_repo() -> Result<(TempDir, RepoManager)> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) = crate::test_support::init_cataloged_repo_with_depth(
        &ledger_dir,
        &tmp_dir.path().join("notes"),
        2,
    )?;
    Ok((tmp_dir, repo))
}

#[test]
fn test_doc_ops_queries_fail_closed_on_dangling_index() -> Result<()> {
    let (_tmp_dir, repo) = new_repo()?;
    let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    let dangling_seq = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let read = db.begin_read()?;
        let table = read.open_multimap_table(DOC_OPS)?;
        table
            .get(doc_id.as_u128())?
            .next_back()
            .transpose()?
            .map(|seq| seq.value())
            .ok_or_else(|| anyhow::anyhow!("missing DOC_OPS entry for {}", doc_id))
    })?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write.open_table(LEDGER_OPS)?.remove(dangling_seq)?;
        write.commit()?;
        Ok(())
    })?;

    for result in [
        repo.run_on_local_repo(repo.local_repo_name(), |db| get_ops_from_db(db, doc_id)),
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            get_ops_from_db_after(db, doc_id, 0)
        }),
    ] {
        let err = result.expect_err("dangling DOC_OPS index must fail closed");
        assert!(err.to_string().contains("Broken DOC_OPS index"));
        assert!(err.to_string().contains("missing ledger op"));
    }

    for result in [
        repo.run_on_local_repo(repo.local_repo_name(), |db| count_ops_from_db(db, doc_id)),
        repo.run_on_local_repo(repo.local_repo_name(), |db| max_seq_from_db(db, doc_id)),
    ] {
        let err = result.expect_err("dangling DOC_OPS index must fail closed");
        assert!(err.to_string().contains("Broken DOC_OPS index"));
    }
    Ok(())
}

#[test]
fn test_node_ops_queries_fail_closed_on_dangling_index() -> Result<()> {
    let (_tmp_dir, repo) = new_repo()?;
    let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    let node_id = NodeId::from_doc_id(doc_id);
    let dangling_seq = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let read = db.begin_read()?;
        let table = read.open_multimap_table(NODE_OPS)?;
        table
            .get(node_id.as_u128())?
            .next_back()
            .transpose()?
            .map(|seq| seq.value())
            .ok_or_else(|| anyhow::anyhow!("missing NODE_OPS entry for {}", node_id))
    })?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write.open_table(LEDGER_OPS)?.remove(dangling_seq)?;
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            get_structure_ops_for_node_from_db(db, node_id)
        })
        .expect_err("dangling NODE_OPS index must fail closed");
    assert!(err.to_string().contains("Broken NODE_OPS index"));
    assert!(err.to_string().contains("missing ledger op"));
    Ok(())
}
