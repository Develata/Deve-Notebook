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
