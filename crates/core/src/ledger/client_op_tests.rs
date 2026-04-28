//! plan_ref:
//!   - 04_storage#facts-partition
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use super::*;
use crate::ledger::schema::{CLIENT_OP_INDEX, LEDGER_OPS};
use crate::models::{DocId, LedgerEntry, PeerId};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn test_find_client_op_in_local_repo() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, None, None)?;
    let doc_id = DocId::new();
    let peer_id = PeerId::new("browser-peer");

    repo.append_generated_client_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        peer_id.clone(),
        42,
        9,
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                crate::models::Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1000,
                peer_id.clone(),
                seq,
                Some(42),
                Some(9),
            )
        },
    )?;

    let found = repo
        .find_client_op_in_local_repo(repo.local_repo_name(), doc_id, 42, 9)?
        .expect("client op should be indexed");
    assert_eq!(found.1.seq, 1);
    assert_eq!(
        found.1.content_op(),
        Some(&crate::models::Op::Insert {
            pos: 0,
            content: "hello".into(),
        })
    );
    assert_eq!(found.1.client_id, Some(42));
    assert_eq!(found.1.client_op_id, Some(9));
    Ok(())
}

#[test]
fn test_find_client_op_fails_closed_when_index_table_is_missing() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, None, None)?;
    let doc_id = DocId::new();

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .find_client_op_in_local_repo(repo.local_repo_name(), doc_id, 42, 9)
        .expect_err("missing client op index must fail closed");
    assert!(err.to_string().contains("Broken client op index"));
    Ok(())
}

#[test]
fn test_find_client_op_fails_closed_on_dangling_index() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, None, None)?;
    let doc_id = DocId::new();
    let peer_id = PeerId::new("browser-peer");

    repo.append_generated_client_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        peer_id.clone(),
        42,
        9,
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                crate::models::Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1000,
                peer_id.clone(),
                seq,
                Some(42),
                Some(9),
            )
        },
    )?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write.open_table(LEDGER_OPS)?.remove(1)?;
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .find_client_op_in_local_repo(repo.local_repo_name(), doc_id, 42, 9)
        .expect_err("dangling client op index must fail closed");
    assert!(err.to_string().contains("Broken client op index"));
    assert!(err.to_string().contains("missing ledger op"));
    Ok(())
}
