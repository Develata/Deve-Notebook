use crate::models::{DocId, LedgerEntry, NodeId, Op, PeerId, StructureOp};

fn entry(op: Op) -> LedgerEntry {
    LedgerEntry::new_content(DocId::new(), op, 0, PeerId::new("test"), 0, None, None)
}

#[test]
fn reconstruct_utf16_insert_after_emoji() {
    let ops = vec![
        entry(Op::Insert {
            pos: 0,
            content: "A😀B".into(),
        }),
        entry(Op::Insert {
            pos: 3,
            content: "X".into(),
        }),
    ];

    let content = crate::state::reconstruct_content(&ops);
    assert_eq!(content, "A😀XB");
}

#[test]
fn reconstruct_utf16_delete_emoji() {
    let ops = vec![
        entry(Op::Insert {
            pos: 0,
            content: "A😀B".into(),
        }),
        entry(Op::Delete { pos: 1, len: 2 }),
    ];

    let content = crate::state::reconstruct_content(&ops);
    assert_eq!(content, "AB");
}

#[test]
fn try_apply_content_ops_applies_utf16_delta_to_existing_base() {
    let ops = vec![
        Op::Insert {
            pos: 3,
            content: "X".into(),
        },
        Op::Delete { pos: 1, len: 2 },
    ];

    let content = crate::state::try_apply_content_ops("A😀B", &ops).expect("valid ops");
    assert_eq!(content, "AXB");
}

#[test]
fn try_apply_content_ops_rejects_out_of_bounds_delta() {
    let ops = vec![Op::Delete { pos: 2, len: 2 }];

    assert_eq!(crate::state::try_apply_content_ops("abc", &ops), None);
}

#[test]
fn compute_diff_uses_utf16_positions() {
    let ops = crate::state::compute_diff("A😀B", "A😀XB");
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        Op::Insert { pos, content } => {
            assert_eq!(*pos, 3);
            assert_eq!(content.as_str(), "X");
        }
        _ => panic!("expected insert op"),
    }
}

#[test]
fn reconstruct_ignores_structure_events() {
    let doc_id = DocId::new();
    let ops = vec![
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "abc".into(),
            },
            0,
            PeerId::new("test"),
            1,
            None,
            None,
        ),
        LedgerEntry::new_structure(
            StructureOp::RenameNode {
                node_id: NodeId::from_doc_id(doc_id),
                doc_id: Some(doc_id),
                new_name: "renamed.md".to_string(),
            },
            1,
            PeerId::new("test"),
            2,
        ),
    ];

    let content = crate::state::reconstruct_content(&ops);
    assert_eq!(content, "abc");
}

#[test]
fn find_invalid_content_op_detects_out_of_bounds_insert_after_delete() {
    let ops = vec![
        entry(Op::Insert {
            pos: 0,
            content: "abcd".into(),
        }),
        entry(Op::Delete { pos: 1, len: 2 }),
        entry(Op::Insert {
            pos: 4,
            content: "x".into(),
        }),
    ];

    let issue = crate::state::find_invalid_content_op(&ops).expect("issue");
    assert_eq!(
        crate::state::describe_invalid_content_op(&issue),
        "insert beyond end at seq 0: pos=4 current_utf16_len=2"
    );
}

#[test]
fn content_op_validator_accepts_exact_end_and_utf16_boundary() {
    let doc_id = DocId::new();
    let peer_id = PeerId::new("test");
    let mut validator = crate::state::ContentOpValidator::default();

    let ops = [
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "A😀".into(),
            },
            0,
            peer_id.clone(),
            1,
            None,
            None,
        ),
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 3,
                content: "B".into(),
            },
            0,
            peer_id.clone(),
            2,
            None,
            None,
        ),
        LedgerEntry::new_content(
            doc_id,
            Op::Delete { pos: 1, len: 2 },
            0,
            peer_id,
            3,
            None,
            None,
        ),
    ];

    for op in ops {
        assert_eq!(validator.push_entry(&op), None);
    }
}

#[test]
fn content_op_validator_reports_candidate_seq_after_history() {
    let doc_id = DocId::new();
    let peer_id = PeerId::new("test");
    let mut validator = crate::state::ContentOpValidator::default();

    assert_eq!(
        validator.push_entry(&LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "abc".into(),
            },
            0,
            peer_id.clone(),
            10,
            None,
            None,
        )),
        None
    );

    let issue = validator
        .push_entry(&LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 4,
                content: "x".into(),
            },
            0,
            peer_id,
            99,
            None,
            None,
        ))
        .expect("candidate issue");

    assert_eq!(
        crate::state::describe_invalid_content_op(&issue),
        "insert beyond end at seq 99: pos=4 current_utf16_len=3"
    );
}
