use super::*;
use deve_core::models::RepoId;
use deve_core::protocol::{ClientOrigin, ConfirmedOp};

fn repo_id() -> RepoId {
    RepoId::from_u128(1)
}

fn pending_input(
    doc_id: DocId,
    scope_nonce: u64,
    client_id: u64,
    client_op_id: u64,
    base_version: u64,
    op: Op,
) -> PendingLocalEditInput {
    PendingLocalEditInput {
        repo_id: repo_id(),
        doc_id,
        scope_nonce,
        client_id,
        client_op_id,
        base_version,
        op,
    }
}

#[test]
fn reconcile_removes_ops_confirmed_by_history() {
    let doc_id = DocId::from_u128(1);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        pending_input(
            doc_id,
            17,
            11,
            1,
            10,
            Op::Insert {
                pos: 0,
                content: "a".into(),
            },
        ),
    );
    push_pending_edit(
        &mut pending,
        pending_input(doc_id, 17, 11, 2, 10, Op::Delete { pos: 1, len: 1 }),
    );
    let history = vec![
        ConfirmedOp::new(
            11,
            Op::Insert {
                pos: 0,
                content: "a".into(),
            },
            Some(ClientOrigin {
                client_id: 11,
                client_op_id: 1,
            }),
        ),
        ConfirmedOp::new(
            12,
            Op::Delete { pos: 1, len: 1 },
            Some(ClientOrigin {
                client_id: 11,
                client_op_id: 2,
            }),
        ),
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
    push_pending_edit(
        &mut pending,
        pending_input(doc_id, 17, 13, 7, 20, op.clone()),
    );
    let history = vec![ConfirmedOp::new(
        19,
        op,
        Some(ClientOrigin {
            client_id: 13,
            client_op_id: 7,
        }),
    )];
    assert_eq!(reconcile_with_history(&mut pending, doc_id, &history), 0);
    assert_eq!(cloned_ops_for_doc(&pending, doc_id).len(), 1);
}

#[test]
fn reconcile_keeps_entries_without_origin_metadata() {
    let doc_id = DocId::from_u128(3);
    let mut pending = PendingLocalEdits::new();
    let op = Op::Insert {
        pos: 1,
        content: "z".into(),
    };
    push_pending_edit(
        &mut pending,
        pending_input(doc_id, 17, 21, 5, 0, op.clone()),
    );
    let history = vec![ConfirmedOp::new(1, op, None)];
    assert_eq!(reconcile_with_history(&mut pending, doc_id, &history), 0);
    assert_eq!(cloned_ops_for_doc(&pending, doc_id).len(), 1);
}
