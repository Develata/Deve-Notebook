use super::resolve_from_entries;
use crate::models::DocId;
use crate::source_control::{ChangeEntry, ChangeStatus};

#[test]
fn resolve_from_entries_matches_renamed_from_without_doc_id() {
    let entries = vec![
        ChangeEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: None,
            status: ChangeStatus::Deleted,
            has_conflict: false,
        },
        ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: None,
            status: ChangeStatus::Added,
            has_conflict: false,
        },
    ];

    assert_eq!(
        resolve_from_entries(&entries, "notes/old.md", None),
        Some("notes/new.md".into())
    );
}

#[test]
fn resolve_from_entries_prefers_doc_id_when_available() {
    let doc_id = DocId(uuid::Uuid::nil());
    let entries = vec![ChangeEntry {
        path: "notes/new.md".into(),
        renamed_from: Some("notes/old.md".into()),
        doc_id: Some(doc_id),
        status: ChangeStatus::Added,
        has_conflict: false,
    }];

    assert_eq!(
        resolve_from_entries(&entries, "notes/old.md", Some(doc_id)),
        Some("notes/new.md".into())
    );
}

#[test]
fn resolve_from_entries_prefers_live_successor_over_exact_deleted_doc_path() {
    let doc_id = DocId(uuid::Uuid::nil());
    let entries = vec![
        ChangeEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            status: ChangeStatus::Deleted,
            has_conflict: false,
        },
        ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(doc_id),
            status: ChangeStatus::Added,
            has_conflict: false,
        },
    ];

    assert_eq!(
        resolve_from_entries(&entries, "notes/old.md", Some(doc_id)),
        Some("notes/new.md".into())
    );
}

#[test]
fn resolve_from_entries_prefers_rename_successor_when_old_path_reused() {
    let old_doc = DocId(uuid::Uuid::nil());
    let new_doc = DocId(uuid::Uuid::from_u128(1));
    let entries = vec![
        ChangeEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: Some(old_doc),
            status: ChangeStatus::Deleted,
            has_conflict: false,
        },
        ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(old_doc),
            status: ChangeStatus::Added,
            has_conflict: false,
        },
        ChangeEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: Some(new_doc),
            status: ChangeStatus::Added,
            has_conflict: false,
        },
    ];

    assert_eq!(
        resolve_from_entries(&entries, "notes/old.md", None),
        Some("notes/new.md".into())
    );
}

#[test]
fn resolve_from_entries_fails_closed_when_path_only_target_is_ambiguous() {
    let entries = vec![
        ChangeEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: None,
            status: ChangeStatus::Added,
            has_conflict: false,
        },
        ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: None,
            status: ChangeStatus::Added,
            has_conflict: false,
        },
    ];

    assert_eq!(resolve_from_entries(&entries, "notes/old.md", None), None);
}

#[test]
fn resolve_from_entries_fails_closed_when_same_path_maps_to_distinct_docs() {
    let old_doc = DocId(uuid::Uuid::nil());
    let new_doc = DocId(uuid::Uuid::from_u128(1));
    let entries = vec![
        ChangeEntry {
            path: "notes/reused.md".into(),
            renamed_from: None,
            doc_id: Some(old_doc),
            status: ChangeStatus::Deleted,
            has_conflict: false,
        },
        ChangeEntry {
            path: "notes/reused.md".into(),
            renamed_from: None,
            doc_id: Some(new_doc),
            status: ChangeStatus::Added,
            has_conflict: false,
        },
    ];

    assert_eq!(
        resolve_from_entries(&entries, "notes/reused.md", None),
        None
    );
}
