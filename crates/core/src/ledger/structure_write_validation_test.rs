use super::{RepoManager, range};
use crate::ledger::node_meta;
use crate::models::{DocId, LedgerEntry, NodeId, PeerId, StructureOp};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn append_local_structure_op_rejects_missing_parent_reference() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = RepoManager::init(tmp_dir.path().join("ledger"), 2, None, None)?;
    let doc_id = DocId::new();
    let entry = LedgerEntry::new_structure(
        StructureOp::CreateFile {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id,
            parent_id: Some(NodeId::new()),
            name: "broken.md".into(),
        },
        1000,
        PeerId::new("test"),
        1,
    );

    let err = repo
        .append_local_op(&entry)
        .expect_err("missing parent must fail closed");
    assert!(err.to_string().contains("Refusing to append structure op"));
    assert!(err.to_string().contains("missing node"));
    assert_eq!(
        repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?,
        0
    );
    Ok(())
}

#[test]
fn append_generated_structure_event_rejects_move_that_creates_cycle() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = RepoManager::init(tmp_dir.path().join("ledger"), 2, None, None)?;
    repo.apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes/sub", "test")?;
    let (notes_id, sub_id) = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        Ok((
            node_meta::get_node_id(db, "notes")?.expect("notes id"),
            node_meta::get_node_id(db, "notes/sub")?.expect("sub id"),
        ))
    })?;
    let before = repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?;

    let err = repo
        .append_generated_structure_event_in_local_repo(
            repo.local_repo_name(),
            PeerId::new("test"),
            StructureOp::MoveNode {
                node_id: notes_id,
                doc_id: None,
                new_parent_id: Some(sub_id),
            },
            2000,
        )
        .expect_err("cycle must fail closed");
    assert!(err.to_string().contains("create cycle"));
    assert_eq!(
        repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?,
        before
    );
    Ok(())
}
