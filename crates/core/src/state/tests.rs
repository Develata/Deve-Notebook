use crate::models::{DocId, LedgerEntry, Op, PeerId};

fn entry(op: Op) -> LedgerEntry {
    LedgerEntry {
        doc_id: DocId::new(),
        op,
        timestamp: 0,
        peer_id: PeerId::new("test"),
        seq: 0,
    }
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
