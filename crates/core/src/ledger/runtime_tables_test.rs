use super::RepoManager;
use crate::ledger::schema::{CLIENT_OP_INDEX, LEDGER_OPS};
use crate::models::{DocId, LedgerEntry, Op};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn repairs_missing_client_op_index_for_secondary_local_repo_on_runtime_open() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    crate::test_support::create_initialized_local_repo_with_depth(
        &ledger_dir,
        2,
        "wiki",
        "urn:wiki",
    );
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    repo.run_on_local_repo("wiki", |db| {
        let write = db.begin_write()?;
        let _ = write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    repo.append_generated_client_op_in_local_repo("wiki", doc_id, peer_id.clone(), 42, 9, |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "hello".into(),
            },
            1000,
            peer_id.clone(),
            seq,
            Some(42),
            Some(9),
        )
    })?;

    let found = repo
        .find_client_op_in_local_repo("wiki", 42, 9)?
        .expect("client op should be rebuilt and indexed");
    assert_eq!(found.1.doc_id, Some(doc_id));
    assert_eq!(found.1.client_id, Some(42));
    assert_eq!(found.1.client_op_id, Some(9));
    Ok(())
}

#[test]
fn repairs_empty_client_op_index_for_primary_local_repo_on_init() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    repo.append_generated_client_op_in_local_repo("main", doc_id, peer_id.clone(), 42, 9, |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "hello".into(),
            },
            1000,
            peer_id.clone(),
            seq,
            Some(42),
            Some(9),
        )
    })?;
    repo.run_on_local_repo("main", |db| {
        let write = db.begin_write()?;
        let _ = write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    let reopened = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    let found = reopened
        .find_client_op_in_local_repo("main", 42, 9)?
        .expect("primary client op index should be rebuilt during init");
    assert_eq!(found.1.doc_id, Some(doc_id));
    assert_eq!(found.1.client_id, Some(42));
    assert_eq!(found.1.client_op_id, Some(9));
    Ok(())
}

#[test]
fn repairs_partial_non_empty_client_op_index_for_primary_local_repo_on_init() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    let first_doc = DocId::new();
    let second_doc = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    repo.append_generated_client_op_in_local_repo(
        "main",
        first_doc,
        peer_id.clone(),
        42,
        9,
        |seq| {
            LedgerEntry::new_content(
                first_doc,
                Op::Insert {
                    pos: 0,
                    content: "first".into(),
                },
                1000,
                peer_id.clone(),
                seq,
                Some(42),
                Some(9),
            )
        },
    )?;
    repo.append_generated_client_op_in_local_repo(
        "main",
        second_doc,
        peer_id.clone(),
        42,
        10,
        |seq| {
            LedgerEntry::new_content(
                second_doc,
                Op::Insert {
                    pos: 0,
                    content: "second".into(),
                },
                1001,
                peer_id.clone(),
                seq,
                Some(42),
                Some(10),
            )
        },
    )?;
    repo.run_on_local_repo("main", |db| {
        let write = db.begin_write()?;
        {
            let mut client_ops = write.open_table(CLIENT_OP_INDEX)?;
            client_ops.insert((42, 9), 2)?;
            client_ops.remove((42, 10))?;
            client_ops.insert((42, 11), 1)?;
        }
        write.commit()?;
        Ok(())
    })?;

    let reopened = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    let first = reopened
        .find_client_op_in_local_repo("main", 42, 9)?
        .expect("wrong-seq client op index should be repaired");
    let second = reopened
        .find_client_op_in_local_repo("main", 42, 10)?
        .expect("missing client op should be rebuilt");
    assert_eq!(first.1.doc_id, Some(first_doc));
    assert_eq!(second.1.doc_id, Some(second_doc));
    assert!(
        reopened
            .find_client_op_in_local_repo("main", 42, 11)?
            .is_none(),
        "stale client op index keys must be removed during rebuild"
    );
    Ok(())
}

#[test]
fn rebuild_coalesces_legacy_duplicate_client_op_metadata_to_first_seq() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    repo.append_generated_op_in_local_repo("main", doc_id, peer_id.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "first".into(),
            },
            1000,
            peer_id.clone(),
            seq,
            Some(42),
            Some(9),
        )
    })?;
    repo.append_generated_op_in_local_repo("main", doc_id, peer_id.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 5,
                content: "second".into(),
            },
            1001,
            peer_id.clone(),
            seq,
            Some(42),
            Some(9),
        )
    })?;
    repo.run_on_local_repo("main", |db| {
        let write = db.begin_write()?;
        let _ = write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    let reopened = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    let found = reopened
        .find_client_op_in_local_repo("main", 42, 9)?
        .expect("duplicate client op metadata should map to first durable ack");
    assert_eq!(found.0, 1);
    assert_eq!(
        found.1.content_op(),
        Some(&Op::Insert {
            pos: 0,
            content: "first".into(),
        })
    );
    Ok(())
}

#[test]
fn fails_closed_when_ledger_ops_authority_missing_for_secondary_local_repo() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    crate::test_support::create_initialized_local_repo_with_depth(
        &ledger_dir,
        2,
        "wiki",
        "urn:wiki",
    );
    repo.run_on_local_repo("wiki", |db| {
        let write = db.begin_write()?;
        let _ = write.delete_table(LEDGER_OPS)?;
        write.commit()?;
        Ok(())
    })?;

    let reopened = RepoManager::init(&ledger_dir, 2, Some("main"), Some("urn:main"))?;
    let err = reopened
        .run_on_local_repo("wiki", |_| Ok(()))
        .expect_err("missing ledger_ops authority table must fail closed");
    assert!(
        err.to_string()
            .contains("ledger_ops authority table missing")
    );
    Ok(())
}
