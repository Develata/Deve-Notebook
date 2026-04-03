use super::compute_reconcile_patch;
use crate::models::{DocId, LedgerEntry, Op, PeerId};

fn entry(doc_id: DocId, seq: u64, op: Op) -> LedgerEntry {
    LedgerEntry::new_content(doc_id, op, 0, PeerId::new("test"), seq, None, None)
}

#[test]
fn reconcile_fails_closed_on_invalid_ledger_ranges() {
    let doc_id = DocId::new();
    let ops = vec![
        entry(
            doc_id,
            1,
            Op::Insert {
                pos: 0,
                content: "abcd".into(),
            },
        ),
        entry(doc_id, 2, Op::Delete { pos: 1, len: 2 }),
        entry(
            doc_id,
            3,
            Op::Insert {
                pos: 4,
                content: "x".into(),
            },
        ),
    ];

    let err = compute_reconcile_patch(&ops, "axd").expect_err("invalid ledger must fail");
    assert!(
        err.to_string()
            .contains("insert beyond end at seq 3: pos=4 current_utf16_len=2")
    );
}
