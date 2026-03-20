use super::select_entry_without_doc;
use crate::models::DocId;
use crate::source_control::{ChangeStatus, staging::StagedEntry};

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

    assert!(select_entry_without_doc(entries, "notes/old.md").is_none());
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

    assert!(select_entry_without_doc(entries, "notes/old.md").is_none());
}
