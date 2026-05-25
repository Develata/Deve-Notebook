//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::test_support::resolve_target_path;
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use uuid::Uuid;

#[test]
fn resolve_target_prefers_doc_id_over_stale_path() {
    let doc_id = DocId(Uuid::nil());
    let entries = vec![ChangeEntry {
        path: "notes/new.md".into(),
        renamed_from: Some("notes/old.md".into()),
        doc_id: Some(doc_id),
        status: ChangeStatus::Added,
        has_conflict: false,
    }];
    let target = ScPathTarget {
        path: "notes/old.md".into(),
        doc_id: Some(doc_id),
    };

    assert_eq!(
        resolve_target_path(&entries, &target),
        Some("notes/new.md".into())
    );
}

#[test]
fn resolve_target_matches_renamed_from_without_doc_id() {
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
        resolve_target_path(&entries, &ScPathTarget::from_path("notes/old.md")),
        Some("notes/new.md".into())
    );
}

#[test]
fn resolve_target_prefers_rename_successor_over_reused_old_path() {
    let old_doc = DocId(Uuid::nil());
    let new_doc = DocId(Uuid::from_u128(1));
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
        resolve_target_path(&entries, &ScPathTarget::from_path("notes/old.md")),
        None
    );
}

#[test]
fn resolve_target_keeps_requested_path_when_doc_id_does_not_match_reused_path() {
    let new_doc = DocId(Uuid::from_u128(1));
    let entries = vec![
        ChangeEntry {
            path: "notes/reused.md".into(),
            renamed_from: None,
            doc_id: Some(new_doc),
            status: ChangeStatus::Added,
            has_conflict: false,
        },
        ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(DocId(Uuid::from_u128(2))),
            status: ChangeStatus::Added,
            has_conflict: false,
        },
    ];

    assert_eq!(
        resolve_target_path(
            &entries,
            &ScPathTarget {
                path: "notes/reused.md".into(),
                doc_id: Some(DocId(Uuid::nil())),
            },
        ),
        None
    );
}

#[test]
fn resolve_target_rejects_unrelated_path_even_when_doc_id_matches() {
    let doc_id = DocId(Uuid::nil());
    let entries = vec![ChangeEntry {
        path: "notes/current.md".into(),
        renamed_from: None,
        doc_id: Some(doc_id),
        status: ChangeStatus::Modified,
        has_conflict: false,
    }];

    assert_eq!(
        resolve_target_path(
            &entries,
            &ScPathTarget {
                path: "notes/unrelated.md".into(),
                doc_id: Some(doc_id),
            },
        ),
        None
    );
}
