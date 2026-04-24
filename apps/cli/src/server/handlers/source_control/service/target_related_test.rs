//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use super::super::related_targets;
use deve_core::models::DocId;
use deve_core::protocol::{ScPathTarget, ServerErrorCode};
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use uuid::Uuid;

#[test]
fn related_targets_rejects_tracked_path_only_ambiguity_as_conflict() {
    let doc_id = DocId(Uuid::nil());
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
    let err = related_targets(&entries, &ScPathTarget::from_path("notes/old.md"))
        .expect_err("tracked path-only ambiguity must fail closed");
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
}
