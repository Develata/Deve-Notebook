use super::RepoManager;
use crate::ledger::range;
use crate::ledger::schema::{DOC_OPS, LEDGER_OPS, PEER_FACT_OPS, PEER_FACT_SEQ};
use crate::models::{DocId, LedgerEntry, NodeId, Op, PeerId, StructureOp};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn append_local_op_rejects_global_seq_overflow_without_side_effects() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = init_repo(tmp_dir.path())?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    insert_global_seq_sentinel(&repo)?;

    let err = repo
        .append_local_op(&content_entry(
            doc_id,
            peer_id.clone(),
            1,
            Op::Insert {
                pos: 0,
                content: "overflow".into(),
            },
        ))
        .expect_err("GlobalSeq overflow must fail before side indexes are updated");

    assert!(err.to_string().contains("GlobalSeq overflow"));
    assert_content_side_indexes_empty(&repo, doc_id, &peer_id)?;
    Ok(())
}

#[test]
fn append_generated_op_rejects_global_seq_overflow_without_side_effects() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = init_repo(tmp_dir.path())?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    insert_global_seq_sentinel(&repo)?;

    let err = repo
        .append_generated_op_in_local_repo(repo.local_repo_name(), doc_id, peer_id.clone(), |seq| {
            content_entry(
                doc_id,
                peer_id.clone(),
                seq,
                Op::Insert {
                    pos: 0,
                    content: "overflow".into(),
                },
            )
        })
        .expect_err("GlobalSeq overflow must fail before side indexes are updated");

    assert!(err.to_string().contains("GlobalSeq overflow"));
    assert_content_side_indexes_empty(&repo, doc_id, &peer_id)?;
    Ok(())
}

#[test]
fn append_generated_op_rejects_peer_fact_seq_overflow_without_side_effects() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = init_repo(tmp_dir.path())?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write
            .open_table(PEER_FACT_SEQ)?
            .insert(peer_id.as_str(), u64::MAX)?;
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .append_generated_op_in_local_repo(repo.local_repo_name(), doc_id, peer_id.clone(), |seq| {
            content_entry(
                doc_id,
                peer_id.clone(),
                seq,
                Op::Insert {
                    pos: 0,
                    content: "overflow".into(),
                },
            )
        })
        .expect_err("PeerFactSeq overflow must fail before ledger append");

    assert!(err.to_string().contains("PeerFactSeq overflow"));
    assert_eq!(
        repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?,
        0
    );
    assert_peer_fact_seq(&repo, &peer_id, Some(u64::MAX))?;
    Ok(())
}

#[test]
fn append_generated_structure_event_shares_peer_fact_seq_overflow() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let repo = init_repo(tmp_dir.path())?;
    let peer_id = repo.local_peer_id().clone();
    let node_id = NodeId::new();

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write
            .open_table(PEER_FACT_SEQ)?
            .insert(peer_id.as_str(), u64::MAX)?;
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .append_generated_structure_event_in_local_repo(
            repo.local_repo_name(),
            peer_id.clone(),
            StructureOp::CreateDir {
                node_id,
                parent_id: None,
                name: "overflow".into(),
            },
            2000,
        )
        .expect_err("PeerFactSeq overflow must fail before ledger append");

    assert!(err.to_string().contains("PeerFactSeq overflow"));
    assert_eq!(
        repo.run_on_local_repo(repo.local_repo_name(), range::get_max_seq)?,
        0
    );
    assert_peer_fact_seq(&repo, &peer_id, Some(u64::MAX))?;
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

fn insert_global_seq_sentinel(repo: &RepoManager) -> Result<()> {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write
            .open_table(LEDGER_OPS)?
            .insert(u64::MAX, b"max-seq-sentinel".as_slice())?;
        write.commit()?;
        Ok(())
    })
}

fn assert_content_side_indexes_empty(
    repo: &RepoManager,
    doc_id: DocId,
    peer_id: &PeerId,
) -> Result<()> {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let read = db.begin_read()?;
        let ops = read.open_table(LEDGER_OPS)?;
        let doc_ops = read.open_multimap_table(DOC_OPS)?;
        assert!(ops.get(u64::MAX)?.is_some());
        assert_eq!(doc_ops.get(doc_id.as_u128())?.count(), 0);
        assert_peer_fact_seq_in_read(&read, peer_id, None)?;
        let peer_ops = read.open_table(PEER_FACT_OPS)?;
        assert!(peer_ops.get((peer_id.as_str(), 1))?.is_none());
        Ok(())
    })
}

fn assert_peer_fact_seq(repo: &RepoManager, peer_id: &PeerId, expected: Option<u64>) -> Result<()> {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let read = db.begin_read()?;
        assert_peer_fact_seq_in_read(&read, peer_id, expected)?;
        Ok(())
    })
}

fn assert_peer_fact_seq_in_read(
    read: &redb::ReadTransaction,
    peer_id: &PeerId,
    expected: Option<u64>,
) -> Result<()> {
    match read.open_table(PEER_FACT_SEQ) {
        Ok(peer_seqs) => {
            assert_eq!(
                peer_seqs.get(peer_id.as_str())?.map(|value| value.value()),
                expected
            );
        }
        Err(redb::TableError::TableDoesNotExist(_)) => assert_eq!(expected, None),
        Err(err) => return Err(err.into()),
    }
    Ok(())
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
