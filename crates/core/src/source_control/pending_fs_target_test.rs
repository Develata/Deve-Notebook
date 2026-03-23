use super::{get_for_target, select_entry_for_doc, select_entry_without_doc};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use redb::Database;
use tempfile::{TempDir, tempdir};

fn new_db() -> (TempDir, Database) {
    let dir = tempdir().expect("create tempdir");
    let db = Database::create(dir.path().join("pending.redb")).expect("create db");
    pending_fs::init_table(&db).expect("init pending table");
    (dir, db)
}

#[test]
fn prefers_rename_successor_when_old_path_is_reused() {
    let entries = vec![
        PendingFsEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
        PendingFsEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
        PendingFsEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: String::new(),
            detected_at: 3,
            has_conflict: false,
        },
    ];

    assert_eq!(
        select_entry_without_doc(entries, "notes/old.md")
            .expect("path-only resolution should succeed")
            .expect("rename successor should win")
            .path,
        "notes/new.md"
    );
}

#[test]
fn fails_closed_when_path_only_target_is_ambiguous() {
    let entries = vec![
        PendingFsEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
        PendingFsEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
    ];

    let err = select_entry_without_doc(entries, "notes/old.md")
        .expect_err("ambiguous path-only target must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous pending_fs target: notes/old.md")
    );
}

#[test]
fn fails_closed_when_path_only_target_matches_tracked_entries() {
    let doc_id = DocId(uuid::Uuid::nil());
    let entries = vec![
        PendingFsEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
        PendingFsEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
    ];

    let err = select_entry_without_doc(entries, "notes/old.md")
        .expect_err("tracked path-only target must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous pending_fs target: notes/old.md")
    );
}

#[test]
fn doc_target_prefers_exact_deleted_half_of_rename_pair() {
    let doc_id = DocId(uuid::Uuid::nil());
    let entries = vec![
        PendingFsEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
        PendingFsEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
    ];

    assert_eq!(
        select_entry_for_doc(entries, "notes/old.md", doc_id)
            .expect("exact deleted half should win")
            .path,
        "notes/old.md"
    );
}

#[test]
fn get_for_target_fails_closed_when_exact_path_and_rename_successor_conflict() {
    let (_dir, db) = new_db();
    pending_fs::upsert(
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
    pending_fs::upsert(
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

    let err = get_for_target(&db, &ScPathTarget::from_path("notes/old.md"))
        .expect_err("ambiguous docless target must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous pending_fs target: notes/old.md")
    );
}

#[test]
fn doc_target_finds_exact_docless_delete_by_path() {
    let (_dir, db) = new_db();
    let doc_id = DocId(uuid::Uuid::new_v4());
    pending_fs::upsert(
        &db,
        &PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
    )
    .expect("seed delete");

    let entry = get_for_target(
        &db,
        &ScPathTarget {
            doc_id: Some(doc_id),
            path: "notes/a.md".into(),
        },
    )
    .expect("doc target lookup should succeed")
    .expect("exact docless delete should resolve");
    assert_eq!(entry.path, "notes/a.md");
    assert_eq!(entry.change_type, ChangeStatus::Deleted);
    assert_eq!(entry.doc_id, None);
}

#[test]
fn doc_target_finds_exact_docless_modified_by_path() {
    let (_dir, db) = new_db();
    let doc_id = DocId(uuid::Uuid::new_v4());
    pending_fs::upsert(
        &db,
        &PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Modified,
            content_hash: String::new(),
            detected_at: 1,
            has_conflict: false,
        },
    )
    .expect("seed modify");

    let entry = get_for_target(
        &db,
        &ScPathTarget {
            doc_id: Some(doc_id),
            path: "notes/a.md".into(),
        },
    )
    .expect("doc target lookup should succeed")
    .expect("exact docless modify should resolve");
    assert_eq!(entry.path, "notes/a.md");
    assert_eq!(entry.change_type, ChangeStatus::Modified);
    assert_eq!(entry.doc_id, None);
}
