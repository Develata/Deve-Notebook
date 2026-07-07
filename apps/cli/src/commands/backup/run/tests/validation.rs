//! plan_ref:
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-artifact-protection-contract

use super::super::run_backup_lines;
use super::support::input;

#[test]
fn requires_local_writable_writer_and_protection_evidence() {
    let mut non_local = input();
    non_local.local_writer_identity = "writer-2";
    let err = run_backup_lines(non_local).expect_err("non-local writer rejected");
    assert!(err.to_string().contains("non-local backup writer"));

    let mut unprotected = input();
    unprotected.authenticated = false;
    let err = run_backup_lines(unprotected).expect_err("protection evidence required");
    assert!(err.to_string().contains("--authenticated"));
}

#[test]
fn validates_ledger_range_and_pack_digest() {
    let mut missing_range = input();
    missing_range.ledger_start = None;
    let err = run_backup_lines(missing_range).expect_err("range required");
    assert!(err.to_string().contains("--ledger-start"));

    let mut invalid_digest = input();
    invalid_digest.payload_digest = "abc";
    let err = run_backup_lines(invalid_digest).expect_err("digest rejected");
    assert!(err.to_string().contains("sha256"));
}
