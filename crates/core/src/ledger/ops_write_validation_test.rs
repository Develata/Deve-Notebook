use super::RepoManager;
use crate::ledger::{node_meta, range};
use crate::models::{DocId, LedgerEntry, NodeId, Op, PeerId, StructureOp};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn append_local_op_rejects_out_of_bounds_content_range() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = init_repo(tmp_dir.path())?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    repo.append_local_op(&content_entry(
        doc_id,
        peer_id.clone(),
        1,
        Op::Insert {
            pos: 0,
            content: "abc".into(),
        },
    ))?;
    repo.append_local_op(&content_entry(
        doc_id,
        peer_id.clone(),
        2,
        Op::Delete { pos: 0, len: 3 },
    ))?;

    let err = repo
        .append_local_op(&content_entry(
            doc_id,
            peer_id,
            3,
            Op::Insert {
                pos: 2,
                content: "x".into(),
            },
        ))
        .expect_err("invalid local content op must fail closed");
    assert!(err.to_string().contains("Refusing to append content op"));
    assert!(err.to_string().contains("insert beyond end"));
    assert_eq!(repo.get_local_ops(doc_id)?.len(), 2);
    Ok(())
}

#[test]
fn append_generated_client_op_rejects_out_of_bounds_content_range() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = init_repo(tmp_dir.path())?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    append_client_op(
        &repo,
        doc_id,
        peer_id.clone(),
        1,
        Op::Insert {
            pos: 0,
            content: "abc".into(),
        },
    )?;
    append_client_op(
        &repo,
        doc_id,
        peer_id.clone(),
        2,
        Op::Delete { pos: 0, len: 3 },
    )?;

    let err = append_client_op(
        &repo,
        doc_id,
        peer_id.clone(),
        3,
        Op::Insert {
            pos: 2,
            content: "x".into(),
        },
    )
    .expect_err("invalid generated content op must fail closed");
    assert!(err.to_string().contains("insert beyond end"));
    assert_eq!(
        repo.get_local_ops_in_local_repo(repo.local_repo_name(), doc_id)?
            .len(),
        2
    );
    assert!(
        repo.find_client_op_in_local_repo(repo.local_repo_name(), 7, 3)?
            .is_none()
    );
    Ok(())
}

#[test]
fn append_generated_op_rejects_structure_entries() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = init_repo(tmp_dir.path())?;
    let doc_id = DocId::new();
    let node_id = NodeId::from_doc_id(doc_id);
    let err = repo
        .append_generated_op_in_local_repo(
            repo.local_repo_name(),
            doc_id,
            repo.local_peer_id().clone(),
            |seq| {
                LedgerEntry::new_structure(
                    StructureOp::CreateFile {
                        node_id,
                        doc_id,
                        parent_id: None,
                        name: "bad.md".into(),
                    },
                    1000,
                    repo.local_peer_id().clone(),
                    seq,
                )
            },
        )
        .expect_err("generated content API must reject structure entries");

    assert!(err.to_string().contains("cannot accept structure events"));
    assert_eq!(
        repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?,
        0
    );
    assert!(
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            node_meta::get_node_id(db, "bad.md")
        })?
        .is_none()
    );
    Ok(())
}

fn init_repo(root: &std::path::Path) -> Result<RepoManager> {
    let (repo, _repo_id) = crate::test_support::init_cataloged_repo_with_depth(
        &root.join("ledger"),
        &root.join("notes"),
        2,
    )?;
    Ok(repo)
}

fn append_client_op(
    repo: &RepoManager,
    doc_id: DocId,
    peer_id: PeerId,
    client_op_id: u64,
    op: Op,
) -> Result<(u64, u64)> {
    repo.append_generated_client_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        peer_id.clone(),
        7,
        client_op_id,
        |seq| content_entry(doc_id, peer_id.clone(), seq, op.clone()),
    )
}

fn content_entry(doc_id: DocId, peer_id: PeerId, seq: u64, op: Op) -> LedgerEntry {
    LedgerEntry::new_content(
        doc_id,
        op,
        1000 + seq as i64,
        peer_id,
        seq,
        Some(7),
        Some(seq),
    )
}
