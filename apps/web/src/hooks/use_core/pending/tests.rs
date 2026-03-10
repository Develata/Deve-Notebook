use super::*;

#[test]
fn reconcile_removes_ops_confirmed_by_history() {
    let doc_id = DocId::from_u128(1);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        doc_id,
        11,
        1,
        10,
        Op::Insert {
            pos: 0,
            content: "a".into(),
        },
    );
    push_pending_edit(
        &mut pending,
        doc_id,
        11,
        2,
        10,
        Op::Delete { pos: 1, len: 1 },
    );
    let history = vec![
        (
            11,
            Op::Insert {
                pos: 0,
                content: "a".into(),
            },
        ),
        (12, Op::Delete { pos: 1, len: 1 }),
    ];
    assert_eq!(reconcile_with_history(&mut pending, doc_id, &history), 2);
    assert!(!pending.contains_key(&doc_id));
}

#[test]
fn reconcile_ignores_matches_before_base_version() {
    let doc_id = DocId::from_u128(2);
    let mut pending = PendingLocalEdits::new();
    let op = Op::Insert {
        pos: 3,
        content: "x".into(),
    };
    push_pending_edit(&mut pending, doc_id, 13, 7, 20, op.clone());
    let history = vec![(19, op)];
    assert_eq!(reconcile_with_history(&mut pending, doc_id, &history), 0);
    assert_eq!(cloned_ops_for_doc(&pending, doc_id).len(), 1);
}
