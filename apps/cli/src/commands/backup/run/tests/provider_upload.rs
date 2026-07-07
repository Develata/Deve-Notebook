//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-upload-state-machine-contract
//!   - 06_backup#backup-provider-dispatch-contract
//!   - 06_backup#backup-command-output-contract

use super::super::{run_backup_lines, run_backup_lines_with_uploader};
use super::support::{DIGEST, RecordingUploader, artifact_file, input};

#[test]
fn backup_run_requires_artifact_for_provider_upload() {
    let mut input = input();
    input.dry_run = false;
    let err = run_backup_lines(input).expect_err("artifact required");
    assert!(err.to_string().contains("--artifact"));
}

#[test]
fn backup_run_uploads_verified_encrypted_artifact_with_recording_provider() {
    let (_dir, artifact_path, artifact_bytes, digest) = artifact_file();
    let mut input = input();
    input.dry_run = false;
    input.payload_digest = &digest;
    input.artifact_path = Some(&artifact_path);
    let mut uploader = RecordingUploader::default();

    let lines = run_backup_lines_with_uploader(input, &mut uploader).expect("backup upload");

    assert_eq!(uploader.calls.len(), 1);
    assert_eq!(
        uploader.calls[0].object_path,
        "deve/branches/writer-1/packs/000001.pack.enc"
    );
    assert_eq!(uploader.calls[0].artifact_bytes, artifact_bytes);
    assert!(lines.iter().any(|line| line == "artifact_io=true"));
    assert!(
        lines
            .iter()
            .any(|line| line == "upload_state=RemoteVerified")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == &format!("uploaded_bytes={}", artifact_bytes.len()))
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "provider_metadata_diagnostic_only=true")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == &format!("remote_verified_payload_digest={digest}"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=false")
    );
}

#[test]
fn backup_run_reports_verified_artifact_bytes_not_provider_reported_bytes() {
    let (_dir, artifact_path, artifact_bytes, digest) = artifact_file();
    let mut input = input();
    input.dry_run = false;
    input.payload_digest = &digest;
    input.artifact_path = Some(&artifact_path);
    let mut uploader = RecordingUploader {
        reported_uploaded_bytes: Some(usize::MAX),
        ..RecordingUploader::default()
    };

    let lines = run_backup_lines_with_uploader(input, &mut uploader).expect("backup upload");

    assert_eq!(uploader.calls.len(), 1);
    assert!(
        lines
            .iter()
            .any(|line| line == "upload_state=RemoteVerified")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == &format!("uploaded_bytes={}", artifact_bytes.len()))
    );
    assert!(
        !lines
            .iter()
            .any(|line| line == &format!("uploaded_bytes={}", usize::MAX))
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "provider_metadata_diagnostic_only=true")
    );
}

#[test]
fn backup_run_rejects_artifact_digest_mismatch_before_provider_upload() {
    let (_dir, artifact_path, _artifact_bytes, _digest) = artifact_file();
    let mut input = input();
    input.dry_run = false;
    input.payload_digest = DIGEST;
    input.artifact_path = Some(&artifact_path);
    let mut uploader = RecordingUploader::default();

    let err = run_backup_lines_with_uploader(input, &mut uploader)
        .expect_err("digest mismatch must fail");

    assert!(err.to_string().contains("digest"));
    assert!(uploader.calls.is_empty());
}

#[test]
fn backup_run_does_not_enter_uploaded_when_provider_upload_fails() {
    let (_dir, artifact_path, _artifact_bytes, digest) = artifact_file();
    let mut input = input();
    input.dry_run = false;
    input.payload_digest = &digest;
    input.artifact_path = Some(&artifact_path);
    let mut uploader = RecordingUploader {
        fail: true,
        ..RecordingUploader::default()
    };

    let err = run_backup_lines_with_uploader(input, &mut uploader)
        .expect_err("provider upload failure must fail the command");

    assert!(err.to_string().contains("recording upload failed"));
    assert_eq!(uploader.calls.len(), 1);
}

#[test]
fn backup_run_rejects_remote_verify_mismatch() {
    let (_dir, artifact_path, _artifact_bytes, digest) = artifact_file();
    let mut input = input();
    input.dry_run = false;
    input.payload_digest = &digest;
    input.artifact_path = Some(&artifact_path);
    let mut uploader = RecordingUploader {
        remote_digest_override: Some(DIGEST.to_string()),
        ..RecordingUploader::default()
    };

    let err = run_backup_lines_with_uploader(input, &mut uploader)
        .expect_err("remote digest mismatch must fail before RemoteVerified");

    assert!(err.to_string().contains("remote backup manifest digest"));
    assert_eq!(uploader.calls.len(), 1);
}

#[test]
fn backup_run_rejects_authoritative_provider_metadata() {
    let (_dir, artifact_path, _artifact_bytes, digest) = artifact_file();
    let mut input = input();
    input.dry_run = false;
    input.payload_digest = &digest;
    input.artifact_path = Some(&artifact_path);
    let mut uploader = RecordingUploader {
        authoritative_metadata: true,
        ..RecordingUploader::default()
    };

    let err = run_backup_lines_with_uploader(input, &mut uploader)
        .expect_err("authoritative provider metadata must fail closed");

    assert!(err.to_string().contains("diagnostic-only"));
    assert_eq!(uploader.calls.len(), 1);
}
