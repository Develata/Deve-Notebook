//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!
//! Byte-stable idempotency tests for `upsert`.
//!
//! Guarantees: repeated signals carrying identical semantic fields
//! MUST NOT rewrite the redb row. `detected_at` is preserved from the
//! first write, so the serialized bytes stay byte-identical across
//! any number of re-detections.

use super::{PendingFsEntry, init_table, upsert};
use crate::ledger::schema::PENDING_FS_OPS;
use crate::source_control::ChangeStatus;
use redb::Database;
use tempfile::TempDir;

fn new_db() -> (TempDir, Database) {
    let dir = TempDir::new().unwrap();
    let db = Database::create(dir.path().join("test.redb")).unwrap();
    init_table(&db).unwrap();
    (dir, db)
}

fn read_raw(db: &Database, path: &str) -> Vec<u8> {
    let read_txn = db.begin_read().unwrap();
    let table = read_txn.open_table(PENDING_FS_OPS).unwrap();
    table.get(path).unwrap().unwrap().value().to_vec()
}

fn make_entry(path: &str, detected_at: i64) -> PendingFsEntry {
    PendingFsEntry {
        path: path.to_string(),
        renamed_from: None,
        doc_id: None,
        change_type: ChangeStatus::Modified,
        content_hash: "abc123".to_string(),
        detected_at,
        has_conflict: false,
    }
}

#[test]
fn repeated_identical_signal_preserves_bytes() {
    let (_dir, db) = new_db();
    let first = make_entry("foo/bar.md", 1_000);
    upsert(&db, &first).unwrap();
    let bytes_first = read_raw(&db, "foo/bar.md");

    for t in [2_000, 3_000, 4_000, 5_000] {
        let repeat = make_entry("foo/bar.md", t);
        upsert(&db, &repeat).unwrap();
        let bytes_now = read_raw(&db, "foo/bar.md");
        assert_eq!(
            bytes_now, bytes_first,
            "row must be byte-stable across repeated identical signals"
        );
    }
}

#[test]
fn semantic_change_rewrites_row() {
    let (_dir, db) = new_db();
    upsert(&db, &make_entry("foo/bar.md", 1_000)).unwrap();
    let before = read_raw(&db, "foo/bar.md");

    let mut changed = make_entry("foo/bar.md", 2_000);
    changed.content_hash = "deadbeef".to_string();
    upsert(&db, &changed).unwrap();
    let after = read_raw(&db, "foo/bar.md");

    assert_ne!(before, after, "semantic field change must rewrite the row");
}

#[test]
fn burst_hundred_repeats_is_idempotent() {
    let (_dir, db) = new_db();
    upsert(&db, &make_entry("a.md", 1)).unwrap();
    let baseline = read_raw(&db, "a.md");
    for i in 0..100 {
        upsert(&db, &make_entry("a.md", 10_000 + i)).unwrap();
    }
    assert_eq!(read_raw(&db, "a.md"), baseline);
}
