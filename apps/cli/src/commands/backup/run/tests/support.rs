//! plan_ref:
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-upload-state-machine-contract
//!   - 06_backup#backup-provider-dispatch-contract

use super::super::super::provider_io::BackupPackUploadOutcome;
use super::super::{BackupPackUploadRequest, BackupPackUploader, RunBackupCommandInput};
use deve_core::backup::{BackupDigest, BackupEncryptedPackArtifact};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub(super) const REPO_ID: &str = "11111111-1111-1111-1111-111111111111";
pub(super) const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub(super) fn input() -> RunBackupCommandInput<'static> {
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

#[derive(Default)]
pub(super) struct RecordingUploader {
    pub(super) calls: Vec<RecordedUpload>,
    pub(super) fail: bool,
    pub(super) remote_digest_override: Option<String>,
    pub(super) authoritative_metadata: bool,
    pub(super) reported_uploaded_bytes: Option<usize>,
}

pub(super) struct RecordedUpload {
    pub(super) object_path: String,
    pub(super) artifact_bytes: Vec<u8>,
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
        let remote_digest = self
            .remote_digest_override
            .clone()
            .unwrap_or_else(|| sha256_hex(request.artifact_bytes));
        Ok(BackupPackUploadOutcome {
            uploaded_bytes: self
                .reported_uploaded_bytes
                .unwrap_or(request.artifact_bytes.len()),
            remote_verified_payload_digest: BackupDigest::sha256(remote_digest),
            provider_metadata_is_diagnostic_only: !self.authoritative_metadata,
        })
    }
}

pub(super) fn artifact_file() -> (tempfile::TempDir, PathBuf, Vec<u8>, String) {
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
