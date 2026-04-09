use super::*;

#[test]
fn clearing_last_pending_edit_for_current_doc_requests_navigation_reset() {
    let doc_id = DocId::from_u128(41);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        doc_id,
        11,
        7,
        3,
        Op::Insert {
            pos: 0,
            content: "a".into(),
        },
    );
    assert!(clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(doc_id),
        doc_id,
        7,
    ));
    assert!(!pending.contains_key(&doc_id));
}

#[test]
fn clearing_one_of_many_pending_edits_keeps_navigation_guard() {
    let doc_id = DocId::from_u128(42);
    let mut pending = PendingLocalEdits::new();
    for client_op_id in [7, 8] {
        push_pending_edit(
            &mut pending,
            doc_id,
            11,
            client_op_id,
            3,
            Op::Insert {
                pos: 0,
                content: "a".into(),
            },
        );
    }
    assert!(!clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(doc_id),
        doc_id,
        7,
    ));
    assert_eq!(pending.get(&doc_id).map(Vec::len), Some(1));
}

#[test]
fn clearing_other_doc_pending_edit_does_not_reset_current_navigation_guard() {
    let current_doc = DocId::from_u128(43);
    let other_doc = DocId::from_u128(44);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        other_doc,
        11,
        7,
        3,
        Op::Insert {
            pos: 0,
            content: "a".into(),
        },
    );
    assert!(!clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(current_doc),
        other_doc,
        7,
    ));
    assert!(!pending.contains_key(&other_doc));
}
