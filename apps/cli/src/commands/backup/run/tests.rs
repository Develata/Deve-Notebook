use super::{RunBackupCommandInput, run_backup_lines};

const REPO_ID: &str = "11111111-1111-1111-1111-111111111111";
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn input() -> RunBackupCommandInput<'static> {
    RunBackupCommandInput {
        locator: "s3://bucket-name/deve/",
        repo_id: REPO_ID,
        branch_name: "main",
        writer_identity: "writer-1",
        local_writer_identity: "writer-1",
        credential_ref: "env:DEVE_BACKUP_TOKEN",
        key_ref: "keyring:deve/default-backup-key",
        pack_sequence: 1,
        ledger_start: Some(1),
        ledger_end: Some(1),
        ledger_event_count: 1,
        snapshot_count: 0,
        payload_digest: DIGEST,
        encrypted: true,
        authenticated: true,
        dry_run: true,
    }
}

#[test]
fn plans_writable_backup_run_without_provider_io() {
    let lines = run_backup_lines(input()).expect("backup run dry-run");

    assert!(lines.iter().any(|line| line == "command=BackupBranch"));
    assert!(lines.iter().any(|line| line == "effect=RemoteUpload"));
    assert!(lines.iter().any(|line| line == "artifact_io=false"));
    assert!(
        lines
            .iter()
            .any(|line| line == "credential_ref=env:<redacted>")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "key_ref=keyring:<redacted>")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "pack_object_path=deve/branches/writer-1/packs/000001.pack.enc")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "upload_state=PackEncrypted")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=false")
    );
}

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

#[test]
fn requires_dry_run() {
    let mut input = input();
    input.dry_run = false;
    let err = run_backup_lines(input).expect_err("dry-run required");
    assert!(err.to_string().contains("--dry-run"));
}
