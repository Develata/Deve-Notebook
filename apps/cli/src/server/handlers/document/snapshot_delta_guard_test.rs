//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
//! Snapshot delta guard regression coverage.

use super::snapshot_delta_guard::{
    DeltaChainIssueKind, delta_ops_fit, find_delta_chain_issue, issue_summary,
};
use deve_core::models::Op;
use deve_core::protocol::ConfirmedOp;

fn confirmed(seq: u64, op: Op) -> ConfirmedOp {
    ConfirmedOp::new(seq, op, None)
}

#[test]
fn accepts_valid_delta_chain() {
    let ops = vec![
        confirmed(
            2,
            Op::Insert {
                pos: 2,
                content: "!".into(),
            },
        ),
        confirmed(3, Op::Delete { pos: 1, len: 1 }),
    ];

    assert!(delta_ops_fit("hi", &ops));
}

#[test]
fn rejects_out_of_bounds_insert() {
    let ops = vec![confirmed(
        2,
        Op::Insert {
            pos: 4,
            content: "!".into(),
        },
    )];

    assert!(!delta_ops_fit("hi", &ops));
}

#[test]
fn reports_invalid_seq_and_reason() {
    let ops = vec![confirmed(7, Op::Delete { pos: 1, len: 5 })];

    let issue = find_delta_chain_issue("hi", &ops).expect("issue");
    assert_eq!(issue.seq, 7);
    assert_eq!(
        issue.kind,
        DeltaChainIssueKind::DeleteBeyondEnd {
            pos: 1,
            len: 5,
            end: 6
        }
    );
    assert!(issue_summary(issue).contains("seq 7"));
}
