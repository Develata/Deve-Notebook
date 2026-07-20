//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::ledger::schema::{CLIENT_OP_INDEX, LEDGER_OPS};
use crate::models::{DocId, LedgerEntry};
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn test_find_client_op_in_local_repo() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &tmp_dir.path().join("notes"))?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

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
        .find_client_op_in_local_repo(repo.local_repo_name(), 42, 9)?
        .expect("client op should be indexed");
    assert_eq!(found.1.doc_id, Some(doc_id));
    assert_eq!(found.1.peer_seq.get(), 1);
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
fn test_client_op_index_is_global_for_client_writer() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &tmp_dir.path().join("notes"))?;
    let first_doc = DocId::new();
    let second_doc = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    repo.append_generated_client_op_in_local_repo(
        repo.local_repo_name(),
        first_doc,
        peer_id.clone(),
        42,
        9,
        |seq| {
            LedgerEntry::new_content(
                first_doc,
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

    let err = repo
        .append_generated_client_op_in_local_repo(
            repo.local_repo_name(),
            second_doc,
            peer_id.clone(),
            42,
            9,
            |seq| {
                LedgerEntry::new_content(
                    second_doc,
                    crate::models::Op::Insert {
                        pos: 0,
                        content: "world".into(),
                    },
                    1001,
                    peer_id.clone(),
                    seq,
                    Some(42),
                    Some(9),
                )
            },
        )
        .expect_err("same client op id must not be accepted for another doc");
    assert!(err.to_string().contains("Client op already indexed"));
    let found = repo
        .find_client_op_in_local_repo(repo.local_repo_name(), 42, 9)?
        .expect("client op should remain indexed");
    assert_eq!(found.1.doc_id, Some(first_doc));
    assert!(repo.get_local_ops(second_doc)?.is_empty());
    Ok(())
}

#[test]
fn test_client_op_index_is_repo_scoped() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, main_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &tmp_dir.path().join("main-notes"))?;
    let (_wiki, wiki_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &tmp_dir.path().join("wiki-notes"))?;
    let main_name = main_id.to_string();
    let wiki_name = wiki_id.to_string();
    let main_doc = DocId::new();
    let wiki_doc = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    repo.append_generated_client_op_in_local_repo(
        &main_name,
        main_doc,
        peer_id.clone(),
        42,
        9,
        |seq| {
            LedgerEntry::new_content(
                main_doc,
                crate::models::Op::Insert {
                    pos: 0,
                    content: "main".into(),
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
        &wiki_name,
        wiki_doc,
        peer_id.clone(),
        42,
        9,
        |seq| {
            LedgerEntry::new_content(
                wiki_doc,
                crate::models::Op::Insert {
                    pos: 0,
                    content: "wiki".into(),
                },
                1001,
                peer_id.clone(),
                seq,
                Some(42),
                Some(9),
            )
        },
    )?;

    let main_found = repo
        .find_client_op_in_local_repo(&main_name, 42, 9)?
        .expect("main client op should be indexed");
    let wiki_found = repo
        .find_client_op_in_local_repo(&wiki_name, 42, 9)?
        .expect("wiki client op should be indexed");
    assert_eq!(main_found.1.doc_id, Some(main_doc));
    assert_eq!(wiki_found.1.doc_id, Some(wiki_doc));
    Ok(())
}

#[test]
fn test_find_client_op_fails_closed_when_index_table_is_missing() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &tmp_dir.path().join("notes"))?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        write.delete_table(CLIENT_OP_INDEX)?;
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .find_client_op_in_local_repo(repo.local_repo_name(), 42, 9)
        .expect_err("missing client op index must fail closed");
    assert!(err.to_string().contains("Broken client op index"));
    Ok(())
}

#[test]
fn test_find_client_op_fails_closed_on_metadata_mismatch() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &tmp_dir.path().join("notes"))?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

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
        {
            let mut client_ops = write.open_table(CLIENT_OP_INDEX)?;
            client_ops.insert((42, 10), 1)?;
        }
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .find_client_op_in_local_repo(repo.local_repo_name(), 42, 10)
        .expect_err("mismatched client op metadata must fail closed");
    assert!(err.to_string().contains("metadata mismatch"));
    Ok(())
}

#[test]
fn test_find_client_op_fails_closed_on_dangling_index() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) =
        crate::test_support::init_cataloged_repo(&ledger_dir, &tmp_dir.path().join("notes"))?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

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
        .find_client_op_in_local_repo(repo.local_repo_name(), 42, 9)
        .expect_err("dangling client op index must fail closed");
    assert!(err.to_string().contains("Broken client op index"));
    assert!(err.to_string().contains("missing ledger op"));
    Ok(())
}
