use super::super::provider_io::BackupPackUploadOutcome;
use super::{
    BackupPackUploadRequest, BackupPackUploader, RunBackupCommandInput, run_backup_lines,
    run_backup_lines_with_uploader,
};
use deve_core::backup::BackupEncryptedPackArtifact;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

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
        artifact_path: None,
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
    assert!(lines.iter().any(|line| line == "uploaded_bytes=<none>"));
    assert!(
        lines
            .iter()
            .any(|line| line == "provider_metadata_diagnostic_only=<none>")
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
    assert!(lines.iter().any(|line| line == "upload_state=Uploaded"));
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
            .any(|line| line == "writes_local_authority=false")
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

#[derive(Default)]
struct RecordingUploader {
    calls: Vec<RecordedUpload>,
    fail: bool,
}

struct RecordedUpload {
    object_path: String,
    artifact_bytes: Vec<u8>,
}

impl BackupPackUploader for RecordingUploader {
    fn upload_pack(
        &mut self,
        request: BackupPackUploadRequest<'_>,
    ) -> anyhow::Result<BackupPackUploadOutcome> {
        self.calls.push(RecordedUpload {
            object_path: request.object_path.to_string(),
            artifact_bytes: request.artifact_bytes.to_vec(),
        });
        if self.fail {
            anyhow::bail!("recording upload failed");
        }
        Ok(BackupPackUploadOutcome {
            uploaded_bytes: request.artifact_bytes.len(),
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

fn artifact_file() -> (tempfile::TempDir, PathBuf, Vec<u8>, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let artifact = BackupEncryptedPackArtifact {
        format_version: 1,
        repo_id: REPO_ID.parse().unwrap(),
        writer_identity: "writer-1".into(),
        branch_path: "deve/branches/writer-1".into(),
        pack_sequence: 1,
        nonce: vec![7; 12],
        ciphertext: vec![1, 2, 3],
    };
    let artifact_bytes = serde_json::to_vec(&artifact).expect("artifact bytes");
    let digest = sha256_hex(&artifact_bytes);
    let path = dir.path().join("000001.pack.enc");
    std::fs::write(&path, &artifact_bytes).expect("write artifact");
    (dir, path, artifact_bytes, digest)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
