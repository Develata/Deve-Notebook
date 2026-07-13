use super::*;
use deve_core::models::RepoId;

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
fn clearing_last_pending_edit_for_current_doc_requests_navigation_reset() {
    let doc_id = DocId::from_u128(41);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        pending_input(
            doc_id,
            31,
            11,
            7,
            3,
            Op::Insert {
                pos: 0,
                content: "a".into(),
            },
        ),
    );
    assert!(clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(doc_id),
        Some(repo_id()),
        Some(31),
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
            pending_input(
                doc_id,
                31,
                11,
                client_op_id,
                3,
                Op::Insert {
                    pos: 0,
                    content: "a".into(),
                },
            ),
        );
    }
    assert!(!clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(doc_id),
        Some(repo_id()),
        Some(31),
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
        pending_input(
            other_doc,
            31,
            11,
            7,
            3,
            Op::Insert {
                pos: 0,
                content: "a".into(),
            },
        ),
    );
    assert!(!clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(current_doc),
        Some(repo_id()),
        Some(31),
        other_doc,
        7,
    ));
    assert!(!pending.contains_key(&other_doc));
}

#[test]
fn pending_row_records_repo_scope_time_and_marker() {
    let doc_id = DocId::from_u128(45);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        pending_input(
            doc_id,
            31,
            11,
            7,
            3,
            Op::Insert {
                pos: 2,
                content: "abc".into(),
            },
        ),
    );

    let edit = &pending.get(&doc_id).expect("pending doc")[0];
    assert_eq!(edit.repo_id, repo_id());
    assert_eq!(edit.doc_id, doc_id);
    assert_eq!(edit.scope_nonce, 31);
    assert_eq!(edit.op_marker, "insert:2:3");
    assert!(edit.created_at_ms > 0);
}

#[test]
fn malformed_overlay_row_is_ignored_by_doc_read_helpers() {
    let doc_id = DocId::from_u128(47);
    let other_doc = DocId::from_u128(48);
    let mut pending = PendingLocalEdits::new();
    pending.entry(doc_id).or_default().push(PendingLocalEdit {
        repo_id: repo_id(),
        doc_id: other_doc,
        scope_nonce: 31,
        client_id: 11,
        client_op_id: 7,
        created_at_ms: 1,
        base_version: 3,
        op_marker: "insert:0:1".into(),
        op: Op::Insert {
            pos: 0,
            content: "a".into(),
        },
    });

    assert_eq!(pending_count_for_doc(&pending, doc_id), 0);
    assert!(!has_pending_edits_for_doc(&pending, doc_id));
    assert!(cloned_ops_for_doc(&pending, doc_id).is_empty());
    assert!(cloned_pending_edits_for_doc(&pending, doc_id).is_empty());
}

#[test]
fn malformed_overlay_metadata_is_ignored_by_doc_read_helpers() {
    let doc_id = DocId::from_u128(49);
    let mut pending = PendingLocalEdits::new();
    pending.entry(doc_id).or_default().extend([
        PendingLocalEdit {
            repo_id: repo_id(),
            doc_id,
            scope_nonce: 31,
            client_id: 11,
            client_op_id: 7,
            created_at_ms: 1,
            base_version: 3,
            op_marker: String::new(),
            op: Op::Insert {
                pos: 0,
                content: "a".into(),
            },
        },
        PendingLocalEdit {
            repo_id: repo_id(),
            doc_id,
            scope_nonce: 31,
            client_id: 11,
            client_op_id: 8,
            created_at_ms: -1,
            base_version: 3,
            op_marker: "delete:0:1".into(),
            op: Op::Delete { pos: 0, len: 1 },
        },
    ]);

    assert_eq!(pending_count_for_doc(&pending, doc_id), 0);
    assert!(!has_pending_edits_for_doc(&pending, doc_id));
    assert!(cloned_ops_for_doc(&pending, doc_id).is_empty());
    assert!(cloned_pending_edits_for_doc(&pending, doc_id).is_empty());
}

#[test]
fn clearing_last_valid_edit_ignores_malformed_rows_for_navigation_reset() {
    let doc_id = DocId::from_u128(50);
    let other_doc = DocId::from_u128(51);
    let mut pending = PendingLocalEdits::new();
    pending.entry(doc_id).or_default().push(PendingLocalEdit {
        repo_id: repo_id(),
        doc_id: other_doc,
        scope_nonce: 31,
        client_id: 11,
        client_op_id: 99,
        created_at_ms: 1,
        base_version: 3,
        op_marker: "insert:0:1".into(),
        op: Op::Insert {
            pos: 0,
            content: "a".into(),
        },
    });
    push_pending_edit(
        &mut pending,
        pending_input(doc_id, 31, 11, 7, 3, Op::Delete { pos: 2, len: 4 }),
    );

    assert!(clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(doc_id),
        Some(repo_id()),
        Some(31),
        doc_id,
        7,
    ));
    assert_eq!(pending.get(&doc_id).map(Vec::len), Some(1));
    assert!(!has_pending_edits_for_doc(&pending, doc_id));
}

#[test]
fn clearing_with_other_scope_keeps_pending_edit() {
    let doc_id = DocId::from_u128(46);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        pending_input(doc_id, 31, 11, 7, 3, Op::Delete { pos: 2, len: 4 }),
    );

    assert!(!clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(doc_id),
        Some(repo_id()),
        Some(99),
        doc_id,
        7,
    ));
    assert_eq!(pending.get(&doc_id).map(Vec::len), Some(1));
}

#[test]
fn clearing_last_current_scope_edit_ignores_other_scope_for_navigation_reset() {
    let doc_id = DocId::from_u128(52);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        pending_input(doc_id, 31, 11, 7, 3, Op::Delete { pos: 2, len: 4 }),
    );
    push_pending_edit(
        &mut pending,
        pending_input(doc_id, 32, 11, 8, 3, Op::Delete { pos: 3, len: 2 }),
    );

    assert!(clear_pending_edit_and_check_current_doc_empty(
        &mut pending,
        Some(doc_id),
        Some(repo_id()),
        Some(31),
        doc_id,
        7,
    ));
    assert_eq!(pending.get(&doc_id).map(Vec::len), Some(1));
}

#[test]
fn scoped_read_helpers_ignore_other_repo_or_scope() {
    let doc_id = DocId::from_u128(53);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        pending_input(
            doc_id,
            31,
            11,
            7,
            3,
            Op::Insert {
                pos: 0,
                content: "a".into(),
            },
        ),
    );
    push_pending_edit(
        &mut pending,
        pending_input(
            doc_id,
            32,
            11,
            8,
            3,
            Op::Insert {
                pos: 1,
                content: "b".into(),
            },
        ),
    );

    let scope = PendingScope {
        repo_id: repo_id(),
        scope_nonce: 31,
    };
    assert_eq!(pending_count_for_doc_in_scope(&pending, doc_id, scope), 1);
    assert!(has_pending_edits_for_doc_in_scope(&pending, doc_id, scope));
    assert_eq!(
        cloned_ops_for_doc_in_scope(&pending, doc_id, scope).len(),
        1
    );
    assert_eq!(
        cloned_pending_edits_for_doc_in_scope(&pending, doc_id, scope).len(),
        1
    );
}

#[test]
fn exact_pending_edit_match_requires_scope_and_client_op() {
    let doc_id = DocId::from_u128(54);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        pending_input(doc_id, 31, 11, 7, 3, Op::Delete { pos: 2, len: 4 }),
    );

    assert!(has_pending_edit(
        &pending,
        Some(repo_id()),
        Some(31),
        doc_id,
        7,
    ));
    assert!(!has_pending_edit(
        &pending,
        Some(repo_id()),
        Some(32),
        doc_id,
        7,
    ));
    assert!(!has_pending_edit(
        &pending,
        Some(repo_id()),
        Some(31),
        doc_id,
        8,
    ));
}

#[test]
fn session_restore_rebinds_only_exact_repo_and_previous_scope() {
    let doc_id = DocId::from_u128(80);
    let other_doc = DocId::from_u128(81);
    let repo_id = RepoId::from_u128(80);
    let other_repo = RepoId::from_u128(81);
    let mut pending = PendingLocalEdits::new();
    for (target_repo, target_doc, scope_nonce, client_op_id) in [
        (repo_id, doc_id, 7, 1),
        (repo_id, other_doc, 7, 2),
        (repo_id, doc_id, 8, 3),
        (other_repo, doc_id, 7, 4),
    ] {
        push_pending_edit(
            &mut pending,
            PendingLocalEditInput {
                repo_id: target_repo,
                doc_id: target_doc,
                scope_nonce,
                client_id: 11,
                client_op_id,
                base_version: 0,
                op: Op::Insert {
                    pos: 0,
                    content: client_op_id.to_string().into(),
                },
            },
        );
    }

    assert_eq!(rebind_pending_scope(&mut pending, repo_id, 7, 9), 2);
    let rebound_scope = PendingScope {
        repo_id,
        scope_nonce: 9,
    };
    assert_eq!(
        pending_count_for_doc_in_scope(&pending, doc_id, rebound_scope),
        1
    );
    assert_eq!(
        pending_count_for_doc_in_scope(&pending, other_doc, rebound_scope),
        1
    );
    assert_eq!(
        pending_count_for_doc_in_scope(
            &pending,
            doc_id,
            PendingScope {
                repo_id,
                scope_nonce: 8,
            },
        ),
        1
    );
    assert_eq!(
        pending_count_for_doc_in_scope(
            &pending,
            doc_id,
            PendingScope {
                repo_id: other_repo,
                scope_nonce: 7,
            },
        ),
        1
    );
}

#[test]
fn session_restore_same_nonce_is_a_noop() {
    let doc_id = DocId::from_u128(82);
    let repo_id = RepoId::from_u128(82);
    let mut pending = PendingLocalEdits::new();
    push_pending_edit(
        &mut pending,
        PendingLocalEditInput {
            repo_id,
            doc_id,
            scope_nonce: 7,
            client_id: 11,
            client_op_id: 1,
            base_version: 0,
            op: Op::Insert {
                pos: 0,
                content: "pending".into(),
            },
        },
    );

    assert_eq!(rebind_pending_scope(&mut pending, repo_id, 7, 7), 0);
    assert_eq!(pending_count_for_doc(&pending, doc_id), 1);
}
