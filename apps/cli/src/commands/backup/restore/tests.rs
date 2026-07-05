use super::{RestoreCommandInput, restore_lines, restore_lines_with_downloader};
use crate::commands::backup::provider_io::{
    BACKUP_PACK_MAX_DOWNLOAD_BYTES, BackupPackDownloadOutcome, BackupPackDownloadRequest,
    BackupPackDownloader,
};
use deve_core::backup::{
    BackupArtifactKey, BackupArtifactKind, BackupArtifactProtectionInput, BackupPackArtifactInput,
    BackupProtectionMechanism, encrypt_backup_pack_artifact, parse_backup_key_ref,
    plan_backup_artifact_protection,
};

const REPO_ID: &str = "11111111-1111-1111-1111-111111111111";
const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn input<'a>(
    pack_digests: &'a [String],
    mode: &'a str,
    write_gate: bool,
    dry_run: bool,
) -> RestoreCommandInput<'a> {
    RestoreCommandInput {
        locator: "s3://bucket-name/deve/",
        repo_id: REPO_ID,
        manifest_repo_id: REPO_ID,
        branch: "writer-1",
        manifest_digest: DIGEST_A,
        pack_digests,
        mode,
        write_gate,
        manifest_verified: true,
        packs_downloaded: true,
        packs_decrypted: true,
        dry_run,
        credential_ref: None,
        pack_sequence: None,
        ledger_start: None,
        ledger_end: None,
        ledger_event_count: None,
        snapshot_count: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadRecord {
    object_path: String,
    credential_ref: String,
    max_bytes: usize,
}

#[derive(Default)]
struct RecordingDownloader {
    artifact_bytes: Vec<u8>,
    metadata_is_diagnostic_only: bool,
    requests: Vec<DownloadRecord>,
}

impl RecordingDownloader {
    fn new(artifact_bytes: Vec<u8>) -> Self {
        Self {
            artifact_bytes,
            metadata_is_diagnostic_only: true,
            requests: Vec::new(),
        }
    }

    fn with_authoritative_metadata(mut self) -> Self {
        self.metadata_is_diagnostic_only = false;
        self
    }
}

impl BackupPackDownloader for RecordingDownloader {
    fn download_pack(
        &mut self,
        request: BackupPackDownloadRequest<'_>,
    ) -> anyhow::Result<BackupPackDownloadOutcome> {
        self.requests.push(DownloadRecord {
            object_path: request.object_path.to_string(),
            credential_ref: request.credential_ref.redacted(),
            max_bytes: request.max_bytes,
        });
        Ok(BackupPackDownloadOutcome {
            artifact_bytes: self.artifact_bytes.clone(),
            downloaded_bytes: self.artifact_bytes.len(),
            provider_metadata_is_diagnostic_only: self.metadata_is_diagnostic_only,
        })
    }
}

fn encrypted_pack_fixture() -> (String, Vec<u8>) {
    let key = BackupArtifactKey::from_bytes(&[7; 32]).expect("artifact key");
    let protection = plan_backup_artifact_protection(BackupArtifactProtectionInput {
        artifact_kind: BackupArtifactKind::Pack,
        key_ref: parse_backup_key_ref("keyring:deve/default-backup-key").expect("key ref"),
        encrypted: true,
        authenticated: true,
        mechanism: BackupProtectionMechanism::AeadTag,
    })
    .expect("artifact protection");
    let artifact = encrypt_backup_pack_artifact(BackupPackArtifactInput {
        repo_id: REPO_ID.parse().expect("repo id"),
        writer_identity: "writer-1",
        branch_path: "deve/branches/writer-1",
        pack_sequence: 1,
        protection: &protection,
        key: &key,
        plaintext: br#"{"ledger":["event"],"snapshots":[]}"#,
    })
    .expect("encrypted pack artifact");

    (
        artifact.payload_digest().expect("payload digest").hex,
        artifact.to_bytes().expect("artifact bytes"),
    )
}

fn download_input<'a>(pack_digests: &'a [String]) -> RestoreCommandInput<'a> {
    let mut command = input(pack_digests, "remote-readonly", false, false);
    command.packs_downloaded = false;
    command.packs_decrypted = false;
    command.credential_ref = Some("env:DEVE_BACKUP_CREDENTIALS");
    command.pack_sequence = Some(1);
    command.ledger_start = Some(1);
    command.ledger_end = Some(1);
    command.ledger_event_count = Some(1);
    command.snapshot_count = Some(0);
    command
}

#[test]
fn plans_remote_readonly_restore_flow_without_candidate_admission() {
    let pack_digests = vec![DIGEST_B.to_string()];
    let lines = restore_lines(input(&pack_digests, "remote-readonly", false, true))
        .expect("restore dry-run");

    assert!(lines.iter().any(|line| line == "command=RestoreBackup"));
    assert!(lines.iter().any(|line| line == "effect=RemoteDownload"));
    assert!(lines.iter().any(|line| line == "dry_run=true"));
    assert!(lines.iter().any(|line| line == "artifact_io=false"));
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_flow_state=PacksDecrypted")
    );
    assert!(lines.iter().any(
        |line| line == "candidate_admission=typed_verification_and_decrypted_evidence_required"
    ));
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=false")
    );
}

#[test]
fn explicit_import_requires_write_gate() {
    let pack_digests = vec![DIGEST_B.to_string()];
    let err = restore_lines(input(&pack_digests, "explicit-import", false, true))
        .expect_err("explicit import must require gate");
    assert!(err.to_string().contains("write gate"));

    let lines = restore_lines(input(&pack_digests, "explicit-import", true, true))
        .expect("explicit import dry-run");
    assert!(lines.iter().any(|line| line == "effect=ExplicitImport"));
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=true")
    );
}

#[test]
fn fails_closed_on_repo_mismatch_and_incomplete_evidence() {
    let pack_digests = vec![DIGEST_B.to_string()];
    let mut mismatched = input(&pack_digests, "remote-readonly", false, true);
    mismatched.manifest_repo_id = "22222222-2222-2222-2222-222222222222";
    let err = restore_lines(mismatched).expect_err("repo mismatch must fail closed");
    assert!(err.to_string().contains("repo id"));

    let mut incomplete = input(&pack_digests, "remote-readonly", false, true);
    incomplete.packs_decrypted = false;
    let err = restore_lines(incomplete).expect_err("incomplete evidence must fail closed");
    assert!(err.to_string().contains("--packs-decrypted"));
}

#[test]
fn requires_dry_run_and_known_mode() {
    let pack_digests = vec![DIGEST_B.to_string()];
    let mut command = input(&pack_digests, "remote-readonly", false, false);
    command.packs_decrypted = false;
    let err = restore_lines(command).expect_err("provider download requires credential metadata");
    assert!(err.to_string().contains("--credential-ref"));

    let err =
        restore_lines(input(&pack_digests, "import", false, true)).expect_err("mode rejected");
    assert!(err.to_string().contains("mode must"));
}

#[test]
fn backup_restore_download_verifies_manifest_digest_and_routing() {
    let (digest, artifact_bytes) = encrypted_pack_fixture();
    let pack_digests = vec![digest.clone()];
    let downloaded_bytes = artifact_bytes.len();
    let mut downloader = RecordingDownloader::new(artifact_bytes);
    let lines = restore_lines_with_downloader(download_input(&pack_digests), &mut downloader)
        .expect("provider download restore");

    assert_eq!(
        downloader.requests,
        vec![DownloadRecord {
            object_path: "deve/branches/writer-1/packs/000001.pack.enc".to_string(),
            credential_ref: "env:<redacted>".to_string(),
            max_bytes: BACKUP_PACK_MAX_DOWNLOAD_BYTES,
        }]
    );
    assert!(lines.iter().any(|line| line == "artifact_io=true"));
    assert!(
        lines
            .iter()
            .any(|line| line == &format!("downloaded_bytes={downloaded_bytes}"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "provider_metadata_diagnostic_only=true")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_flow_state=PacksDownloaded")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "candidate_admission=not_created_download_verify_only")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=false")
    );
    assert!(lines.iter().any(|line| line == "verified_pack_sequence=1"));
    assert!(lines.iter().any(|line| {
        line == "verified_pack_object_path=deve/branches/writer-1/packs/000001.pack.enc"
    }));
    assert!(
        lines
            .iter()
            .any(|line| line == &format!("verified_pack_digest={digest}"))
    );
}

#[test]
fn backup_restore_download_stops_before_decrypt_or_candidate() {
    let (digest, artifact_bytes) = encrypted_pack_fixture();
    let pack_digests = vec![digest];
    let mut downloader = RecordingDownloader::new(artifact_bytes);
    let mut command = download_input(&pack_digests);
    command.packs_decrypted = true;
    let err = restore_lines_with_downloader(command, &mut downloader)
        .expect_err("download branch must reject decrypted evidence");

    assert!(err.to_string().contains("stops before"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_download_rejects_tampered_artifact_before_candidate() {
    let (digest, mut artifact_bytes) = encrypted_pack_fixture();
    artifact_bytes.push(b'\n');
    let pack_digests = vec![digest];
    let mut downloader = RecordingDownloader::new(artifact_bytes);
    let err = restore_lines_with_downloader(download_input(&pack_digests), &mut downloader)
        .expect_err("tampered artifact must fail before candidate admission");

    assert!(err.to_string().contains("digest"));
    assert_eq!(downloader.requests.len(), 1);
}

#[test]
fn backup_restore_download_rejects_authoritative_provider_metadata() {
    let (digest, artifact_bytes) = encrypted_pack_fixture();
    let pack_digests = vec![digest];
    let mut downloader = RecordingDownloader::new(artifact_bytes).with_authoritative_metadata();
    let err = restore_lines_with_downloader(download_input(&pack_digests), &mut downloader)
        .expect_err("provider metadata must remain diagnostic only");

    assert!(err.to_string().contains("diagnostic-only"));
    assert_eq!(downloader.requests.len(), 1);
}

#[test]
fn backup_restore_download_rejects_metadata_before_provider_get() {
    let (digest, artifact_bytes) = encrypted_pack_fixture();
    let pack_digests = vec![digest];
    let mut command = download_input(&pack_digests);
    command.manifest_repo_id = "22222222-2222-2222-2222-222222222222";
    let mut downloader = RecordingDownloader::new(artifact_bytes);
    let err = restore_lines_with_downloader(command, &mut downloader)
        .expect_err("manifest mismatch must fail before provider I/O");

    assert!(err.to_string().contains("repo id"));
    assert!(downloader.requests.is_empty());

    let (digest, artifact_bytes) = encrypted_pack_fixture();
    let pack_digests = vec![digest];
    let mut command = download_input(&pack_digests);
    command.manifest_digest = "not-a-sha256-digest";
    let mut downloader = RecordingDownloader::new(artifact_bytes);
    let err = restore_lines_with_downloader(command, &mut downloader)
        .expect_err("invalid manifest digest must fail before provider I/O");

    assert!(err.to_string().contains("digest"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_explicit_import_non_dry_run_remains_fail_closed() {
    let (digest, artifact_bytes) = encrypted_pack_fixture();
    let pack_digests = vec![digest];
    let mut command = download_input(&pack_digests);
    command.mode = "explicit-import";
    command.write_gate = true;
    let mut downloader = RecordingDownloader::new(artifact_bytes);
    let err = restore_lines_with_downloader(command, &mut downloader)
        .expect_err("explicit import execution must remain closed");

    assert!(err.to_string().contains("fail-closed"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_explicit_merge_non_dry_run_remains_fail_closed() {
    let (digest, artifact_bytes) = encrypted_pack_fixture();
    let pack_digests = vec![digest];
    let mut command = download_input(&pack_digests);
    command.mode = "explicit-merge";
    command.write_gate = true;
    let mut downloader = RecordingDownloader::new(artifact_bytes);
    let err = restore_lines_with_downloader(command, &mut downloader)
        .expect_err("explicit merge execution must remain closed");

    assert!(err.to_string().contains("fail-closed"));
    assert!(downloader.requests.is_empty());
}
