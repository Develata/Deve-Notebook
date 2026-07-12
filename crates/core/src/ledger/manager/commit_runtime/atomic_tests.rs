//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#facts-partition

use super::*;
use crate::ledger::schema::DOC_OPS;
use crate::models::{LedgerEntry, Op, PeerId, serialize_ledger_entry};
use tempfile::TempDir;

fn database() -> Result<(TempDir, Database)> {
    let dir = tempfile::tempdir()?;
    let db = Database::create(dir.path().join("commit-state.redb"))?;
    changes::init_table(&db)?;
    commits::init_table(&db)?;
    let write_txn = db.begin_write()?;
    let _ = write_txn.open_table(LEDGER_OPS)?;
    let _ = write_txn.open_multimap_table(DOC_OPS)?;
    write_txn.commit()?;
    Ok((dir, db))
}

fn seed_content_fact(db: &Database, doc_id: DocId, seq: u64, content: &str) -> Result<()> {
    let entry = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 0,
            content: content.into(),
        },
        1,
        PeerId::new("commit-test"),
        seq,
        None,
        None,
    );
    let bytes = serialize_ledger_entry(&entry)?;
    let write_txn = db.begin_write()?;
    write_txn
        .open_table(LEDGER_OPS)?
        .insert(seq, bytes.as_slice())?;
    write_txn
        .open_multimap_table(DOC_OPS)?
        .insert(doc_id.as_u128(), seq)?;
    write_txn.commit()?;
    Ok(())
}

#[test]
fn commit_state_second_step_failure_rolls_back_snapshots() -> Result<()> {
    let (_dir, db) = database()?;
    let doc_id = DocId::new();
    changes::save_snapshot(&db, doc_id, "old")?;
    seed_content_fact(&db, doc_id, 1, "new")?;

    let error = persist_commit_state_atomically_with_hook(
        &db,
        &[SnapshotMutation::Save(doc_id)],
        1,
        None,
        "commit",
        1,
        |step| match step {
            CommitStateStep::BeforeAnchor => anyhow::bail!("injected anchor failure"),
            CommitStateStep::Snapshot(_) => Ok(()),
        },
    )
    .expect_err("anchor failure must abort snapshot transaction");

    assert!(error.to_string().contains("injected anchor failure"));
    assert_eq!(
        changes::get_committed_content(&db, doc_id)?.as_deref(),
        Some("old")
    );
    assert!(commits::list(&db, 10)?.is_empty());
    Ok(())
}

#[test]
fn commit_state_multi_doc_snapshot_failure_rolls_back_entire_transaction() -> Result<()> {
    let (_dir, db) = database()?;
    let first = DocId::new();
    let second = DocId::new();
    changes::save_snapshot(&db, first, "old-first")?;
    changes::save_snapshot(&db, second, "old-second")?;
    seed_content_fact(&db, first, 1, "new-first")?;
    seed_content_fact(&db, second, 2, "new-second")?;

    persist_commit_state_atomically_with_hook(
        &db,
        &[
            SnapshotMutation::Save(first),
            SnapshotMutation::Save(second),
        ],
        2,
        None,
        "commit",
        2,
        |step| match step {
            CommitStateStep::Snapshot(0) => anyhow::bail!("injected middle snapshot failure"),
            _ => Ok(()),
        },
    )
    .expect_err("middle snapshot failure must roll back all writes");

    assert_eq!(
        changes::get_committed_content(&db, first)?.as_deref(),
        Some("old-first")
    );
    assert_eq!(
        changes::get_committed_content(&db, second)?.as_deref(),
        Some("old-second")
    );
    assert!(commits::list(&db, 10)?.is_empty());
    Ok(())
}

#[test]
fn commit_state_success_commits_snapshots_and_anchor_atomically() -> Result<()> {
    let (_dir, db) = database()?;
    let saved = DocId::new();
    let removed = DocId::new();
    changes::save_snapshot(&db, removed, "remove-me")?;
    seed_content_fact(&db, saved, 1, "current")?;

    let commit = persist_commit_state_atomically(
        &db,
        &[
            SnapshotMutation::Save(saved),
            SnapshotMutation::Remove(removed),
        ],
        1,
        None,
        "commit",
        2,
    )?;

    assert_eq!(
        changes::get_committed_content(&db, saved)?.as_deref(),
        Some("current")
    );
    assert!(changes::get_committed_content(&db, removed)?.is_none());
    let commits = commits::list(&db, 10)?;
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].id, commit.id);
    assert_eq!(commits[0].ledger_seq, commit.ledger_seq);
    Ok(())
}

#[test]
fn commit_state_rejects_ledger_head_drift_before_any_write() -> Result<()> {
    let (_dir, db) = database()?;
    let doc_id = DocId::new();
    changes::save_snapshot(&db, doc_id, "old")?;
    let write_txn = db.begin_write()?;
    write_txn.open_table(LEDGER_OPS)?.insert(1, &[0_u8][..])?;
    write_txn.commit()?;

    let error = persist_commit_state_atomically(
        &db,
        &[SnapshotMutation::Save(doc_id)],
        0,
        None,
        "commit",
        1,
    )
    .expect_err("ledger head drift must abort commit state transaction");

    assert!(error.to_string().contains("ledger head changed"));
    assert_eq!(
        changes::get_committed_content(&db, doc_id)?.as_deref(),
        Some("old")
    );
    assert!(commits::list(&db, 10)?.is_empty());
    Ok(())
}

#[test]
fn commit_state_rejects_parent_drift_before_any_write() -> Result<()> {
    let (_dir, db) = database()?;
    let doc_id = DocId::new();
    changes::save_snapshot(&db, doc_id, "old")?;
    let concurrent = commits::create(&db, "concurrent", 0, 0)?;

    let error = persist_commit_state_atomically(
        &db,
        &[SnapshotMutation::Save(doc_id)],
        0,
        None,
        "stale commit",
        1,
    )
    .expect_err("parent drift must abort commit state transaction");

    assert!(error.to_string().contains("commit parent changed"));
    assert_eq!(
        changes::get_committed_content(&db, doc_id)?.as_deref(),
        Some("old")
    );
    let commits = commits::list(&db, 10)?;
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].id, concurrent.id);
    Ok(())
}
