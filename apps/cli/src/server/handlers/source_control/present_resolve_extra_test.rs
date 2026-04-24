//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use super::test_support::resolve_target_path;
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
