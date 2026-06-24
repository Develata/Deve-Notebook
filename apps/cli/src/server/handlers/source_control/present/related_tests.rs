//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::test_support::expand_related_targets;
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use uuid::Uuid;

#[test]
fn expand_related_targets_preserves_doc_id_for_rename_pair() {
    let doc_id = DocId(Uuid::nil());
    let entries = vec![
        ChangeEntry {
            path: "notes/old.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            status: ChangeStatus::Deleted,
            has_conflict: false,
            domain: Default::default(),
            base_seq: None,
            target_seq: None,
        },
        ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(doc_id),
            status: ChangeStatus::Added,
            has_conflict: false,
            domain: Default::default(),
            base_seq: None,
            target_seq: None,
        },
    ];

    assert_eq!(
        expand_related_targets(
            &entries,
            &ScPathTarget {
                path: "notes/new.md".into(),
                doc_id: Some(doc_id),
                domain: None,
            },
        ),
        vec![
            ScPathTarget {
                path: "notes/new.md".into(),
                doc_id: Some(doc_id),
                domain: None,
            },
            ScPathTarget {
                path: "notes/old.md".into(),
                doc_id: Some(doc_id),
                domain: None,
            },
        ]
    );
}
