//! plan_ref:
//!   - 04_repository#tree-projection-contract

use super::*;
use crate::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use crate::ledger::{node_meta, range};
use tempfile::TempDir;

#[test]
fn append_local_structure_batch_rolls_back_prefix_on_failure() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = RepoManager::init(tmp_dir.path().join("ledger"), 2, None, None)?;
    let ops = vec![
        StructureOp::CreateDir {
            node_id: NodeId::new(),
            parent_id: None,
            name: "kept-out".into(),
        },
        StructureOp::CreateDir {
            node_id: NodeId::new(),
            parent_id: Some(NodeId::new()),
            name: "broken".into(),
        },
    ];

    let err = repo
        .append_structure_ops_in_local_repo(repo.local_repo_name(), "test", &ops)
        .expect_err("batch failure must fail closed");
    assert!(err.to_string().contains("Refusing to append structure op"));
    assert!(err.to_string().contains("missing node"));
    assert_eq!(
        repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?,
        0
    );
    let leaked = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        node_meta::get_node_id(db, "kept-out")
    })?;
    assert!(leaked.is_none());
    Ok(())
}

#[test]
fn append_local_structure_batch_rolls_back_on_projection_failure() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = RepoManager::init(tmp_dir.path().join("ledger"), 2, None, None)?;
    let (doc_id, _) =
        repo.apply_file_structure_in_local_repo(repo.local_repo_name(), "old.md", None, "test")?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        write_txn.open_table(PATH_TO_DOCID)?.remove("old.md")?;
        write_txn
            .open_table(DOCID_TO_PATH)?
            .remove(doc_id.as_u128())?;
        write_txn.commit()?;
        Ok(())
    })?;
    let before_seq = repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?;
    let prefix_id = NodeId::new();
    let ops = vec![
        StructureOp::CreateDir {
            node_id: prefix_id,
            parent_id: None,
            name: "before-failure".into(),
        },
        StructureOp::RenameNode {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id: Some(doc_id),
            new_name: "new.md".into(),
        },
    ];

    let err = repo
        .append_structure_ops_in_local_repo(repo.local_repo_name(), "test", &ops)
        .expect_err("projection failure must fail closed");
    assert!(err.to_string().contains("Document not found in ledger"));
    assert_eq!(
        repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?,
        before_seq
    );
    assert!(
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            node_meta::get_node_id(db, "before-failure")
        })?
        .is_none()
    );
    assert!(
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            node_meta::get_node_id(db, "new.md")
        })?
        .is_none()
    );
    Ok(())
}
