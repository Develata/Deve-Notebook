//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::{resolve_target_path_strict, test_support::resolve_target_path};
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use uuid::Uuid;

#[test]
fn resolve_target_fails_closed_when_path_only_resolution_is_ambiguous() {
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

    assert_eq!(
        resolve_target_path(&entries, &ScPathTarget::from_path("notes/old.md")),
        None
    );
}

#[test]
fn resolve_target_fails_closed_when_same_path_maps_to_distinct_docs() {
    let old_doc = DocId(Uuid::nil());
    let new_doc = DocId(Uuid::from_u128(1));
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
        resolve_target_path(&entries, &ScPathTarget::from_path("notes/reused.md")),
        None
    );
}

#[test]
fn resolve_target_fails_closed_when_doc_id_has_multiple_rename_successors() {
    let doc_id = DocId(Uuid::nil());
    let entries = vec![
        ChangeEntry {
            path: "notes/new-a.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(doc_id),
            status: ChangeStatus::Added,
            has_conflict: false,
        },
        ChangeEntry {
            path: "notes/new-b.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(doc_id),
            status: ChangeStatus::Added,
            has_conflict: false,
        },
    ];

    let err = resolve_target_path_strict(
        &entries,
        &ScPathTarget {
            path: "notes/old.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .expect_err("multiple doc rename successors must fail closed");

    assert!(
        err.to_string()
            .contains("matched multiple doc rename successors")
    );
}

#[test]
fn resolve_target_fails_closed_when_doc_id_matches_exact_and_successor() {
    let doc_id = DocId(Uuid::nil());
    let entries = vec![
        ChangeEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            status: ChangeStatus::Modified,
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

    let err = resolve_target_path_strict(
        &entries,
        &ScPathTarget {
            path: "notes/old.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .expect_err("exact doc entry plus rename successor must fail closed");

    assert!(
        err.to_string()
            .contains("matched exact doc entry and rename successor")
    );
}
