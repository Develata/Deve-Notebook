use super::RepoManager;
use crate::models::{DocId, LedgerEntry, Op, PeerId};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn append_local_op_rejects_out_of_bounds_content_range() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = init_repo(tmp_dir.path())?;
    let doc_id = DocId::new();
    let peer_id = PeerId::new("local_watcher");

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
    let peer_id = PeerId::new("browser");

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
        repo.find_client_op_in_local_repo(repo.local_repo_name(), doc_id, 7, 3)?
            .is_none()
    );
    Ok(())
}

fn init_repo(root: &std::path::Path) -> Result<RepoManager> {
    RepoManager::init(root.join("ledger"), 2, None, None)
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
