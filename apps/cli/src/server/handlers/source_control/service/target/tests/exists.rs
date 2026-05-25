//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::target_exists;
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use uuid::Uuid;

#[test]
fn target_exists_rejects_docless_match_against_tracked_entry() {
    let entries = vec![ChangeEntry {
        path: "notes/a.md".into(),
        renamed_from: None,
        doc_id: Some(DocId(Uuid::nil())),
        status: ChangeStatus::Modified,
        has_conflict: false,
    }];
    assert!(!target_exists(
        &entries,
        &ScPathTarget::from_path("notes/a.md")
    ));
}

#[test]
fn target_exists_accepts_docless_match_for_untracked_entry() {
    let entries = vec![ChangeEntry {
        path: "notes/a.md".into(),
        renamed_from: None,
        doc_id: None,
        status: ChangeStatus::Added,
        has_conflict: false,
    }];
    assert!(target_exists(
        &entries,
        &ScPathTarget::from_path("notes/a.md")
    ));
}
