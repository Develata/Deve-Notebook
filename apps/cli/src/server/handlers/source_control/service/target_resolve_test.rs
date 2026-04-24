//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use super::super::resolve_target;
use deve_core::models::DocId;
use deve_core::protocol::{ScPathTarget, ServerErrorCode};
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use uuid::Uuid;

#[test]
fn rejects_unresolved_path_only_target() {
    let err = resolve_target(&[], &ScPathTarget::from_path("notes/missing.md"))
        .expect_err("missing target must fail closed");
    assert_eq!(err.code, ServerErrorCode::ScConflictTargetMissing);
}

#[test]
fn accepts_path_only_rename_successor_when_entry_exists() {
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
        resolve_target(&entries, &ScPathTarget::from_path("notes/old.md"))
            .expect("rename successor should resolve"),
        ScPathTarget::from_path("notes/new.md")
    );
}

#[test]
fn rejects_path_only_tracked_rename_successor_as_conflict() {
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
    let err = resolve_target(&entries, &ScPathTarget::from_path("notes/old.md"))
        .expect_err("tracked path-only target must fail closed");
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
}

#[test]
fn rejects_path_reuse_conflict_as_storage_conflict() {
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
    let err = resolve_target(&entries, &ScPathTarget::from_path("notes/old.md"))
        .expect_err("ambiguous path-only target must fail closed");
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
}
