use super::*;
use deve_core::models::DocId;
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
fn expand_related_targets_preserves_doc_id_for_rename_pair() {
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

    assert_eq!(
        expand_related_targets(
            &entries,
            &ScPathTarget {
                path: "notes/new.md".into(),
                doc_id: Some(doc_id),
            },
        ),
        vec![
            ScPathTarget {
                path: "notes/new.md".into(),
                doc_id: Some(doc_id),
            },
            ScPathTarget {
                path: "notes/old.md".into(),
                doc_id: Some(doc_id),
            },
        ]
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
