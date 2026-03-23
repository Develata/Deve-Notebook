use super::{PendingFsEntry, get_for_target, new_db};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs;

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
