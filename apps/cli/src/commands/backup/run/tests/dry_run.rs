//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 06_backup#backup-command-output-contract
//!   - 06_backup#backup-upload-state-machine-contract

use super::super::run_backup_lines;
use super::support::input;

#[test]
fn plans_writable_backup_run_without_provider_io() {
    let lines = run_backup_lines(input()).expect("backup run dry-run");

    assert!(lines.iter().any(|line| line == "command=BackupBranch"));
    assert!(lines.iter().any(|line| line == "effect=RemoteUpload"));
    assert!(lines.iter().any(|line| line == "artifact_io=false"));
    assert!(lines.iter().any(|line| line == "uploaded_bytes=<none>"));
    assert!(
        lines
            .iter()
            .any(|line| line == "provider_metadata_diagnostic_only=<none>")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "remote_verified_payload_digest=<none>")
    );
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
