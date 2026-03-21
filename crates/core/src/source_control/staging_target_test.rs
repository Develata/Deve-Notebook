use super::{get_staged_for_target, select_entry_without_doc};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{
    ChangeStatus, pending_fs::PendingFsEntry, staging, staging::StagedEntry,
};
use redb::Database;
use tempfile::{TempDir, tempdir};

fn new_db() -> (TempDir, Database) {
    let dir = tempdir().expect("create tempdir");
    let db = Database::create(dir.path().join("staging.redb")).expect("create db");
    staging::init_table(&db).expect("init staged table");
    (dir, db)
}

#[test]
fn prefers_rename_successor_when_old_path_is_reused() {
    let entries = vec![
        (
            "notes/old.md".into(),
            StagedEntry {
                timestamp: 1,
                doc_id: None,
                status: ChangeStatus::Deleted,
                content_hash: String::new(),
                has_conflict: false,
                renamed_from: None,
            },
        ),
        (
            "notes/new.md".into(),
            StagedEntry {
                timestamp: 2,
                doc_id: None,
                status: ChangeStatus::Added,
                content_hash: String::new(),
                has_conflict: false,
                renamed_from: Some("notes/old.md".into()),
            },
        ),
        (
            "notes/old.md".into(),
            StagedEntry {
                timestamp: 3,
                doc_id: None,
                status: ChangeStatus::Added,
                content_hash: String::new(),
                has_conflict: false,
                renamed_from: None,
            },
        ),
    ];

    assert_eq!(
        select_entry_without_doc(entries, "notes/old.md")
            .expect("path-only resolution should succeed")
            .expect("rename successor should win")
            .0,
        "notes/new.md"
    );
}

#[test]
fn fails_closed_when_path_only_target_is_ambiguous() {
    let entries = vec![
        (
            "notes/old.md".into(),
            StagedEntry {
                timestamp: 1,
                doc_id: None,
                status: ChangeStatus::Added,
                content_hash: String::new(),
                has_conflict: false,
                renamed_from: None,
            },
        ),
        (
            "notes/new.md".into(),
            StagedEntry {
                timestamp: 2,
                doc_id: None,
                status: ChangeStatus::Added,
                content_hash: String::new(),
                has_conflict: false,
                renamed_from: Some("notes/old.md".into()),
            },
        ),
    ];

    let err = select_entry_without_doc(entries, "notes/old.md")
        .expect_err("ambiguous path-only target must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous staged target: notes/old.md")
    );
}

#[test]
fn fails_closed_when_path_only_target_matches_tracked_entries() {
    let doc_id = DocId(uuid::Uuid::nil());
    let entries = vec![
        (
            "notes/old.md".into(),
            StagedEntry {
                timestamp: 1,
                doc_id: Some(doc_id),
                status: ChangeStatus::Deleted,
                content_hash: String::new(),
                has_conflict: false,
                renamed_from: None,
            },
        ),
        (
            "notes/new.md".into(),
            StagedEntry {
                timestamp: 2,
                doc_id: Some(doc_id),
                status: ChangeStatus::Added,
                content_hash: String::new(),
                has_conflict: false,
                renamed_from: Some("notes/old.md".into()),
            },
        ),
    ];

    let err = select_entry_without_doc(entries, "notes/old.md")
        .expect_err("tracked path-only target must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous staged target: notes/old.md")
    );
}

#[test]
fn get_staged_for_target_fails_closed_when_exact_path_and_rename_successor_conflict() {
    let (_dir, db) = new_db();
    staging::stage_pending_entry(
        &db,
        &PendingFsEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
    )
    .expect("seed exact");
    staging::stage_pending_entry(
        &db,
        &PendingFsEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
    )
    .expect("seed rename successor");

    let err = get_staged_for_target(&db, &ScPathTarget::from_path("notes/old.md"))
        .expect_err("ambiguous staged docless target must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous staged target: notes/old.md")
    );
}
