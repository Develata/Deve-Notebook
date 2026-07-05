use super::{RestoreCommandInput, restore_lines, restore_lines_with_runtime};
use crate::commands::backup::provider_io::{
    BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES, BackupArtifactDownloadOutcome,
    BackupArtifactDownloadRequest, BackupArtifactDownloader, BackupArtifactKeyResolver,
};
use deve_core::backup::{
    BACKUP_RESTORE_MAX_PACKS, BackupArtifactKey, BackupArtifactKind, BackupArtifactProtectionInput,
    BackupBranchManifestArtifactInput, BackupBranchManifestPackRef, BackupLocator,
    BackupProtectionMechanism, BackupSecretRef, encrypt_backup_branch_manifest_artifact,
    encrypt_backup_pack_artifact, parse_backup_key_ref, plan_backup_artifact_protection,
};
use std::collections::HashMap;

const REPO_ID: &str = "11111111-1111-1111-1111-111111111111";
const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

type ArtifactMap = Vec<(String, Vec<u8>)>;
type DownloadFixture = (BackupArtifactKey, String, ArtifactMap, Vec<String>);
type RestoreFlagSetter = fn(&mut RestoreCommandInput<'_>);
type ForbiddenFlagCase = (&'static str, RestoreFlagSetter);

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
        key_ref: None,
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

#[derive(Debug, Default)]
struct RecordingDownloader {
    artifacts: HashMap<String, Vec<u8>>,
    metadata_is_diagnostic_only: bool,
    requests: Vec<DownloadRecord>,
}

impl RecordingDownloader {
    fn new(artifacts: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
        Self {
            artifacts: artifacts.into_iter().collect(),
            metadata_is_diagnostic_only: true,
            requests: Vec::new(),
        }
    }

    fn with_authoritative_metadata(mut self) -> Self {
        self.metadata_is_diagnostic_only = false;
        self
    }
}

impl BackupArtifactDownloader for RecordingDownloader {
    fn download_artifact(
        &mut self,
        request: BackupArtifactDownloadRequest<'_>,
    ) -> anyhow::Result<BackupArtifactDownloadOutcome> {
        self.requests.push(DownloadRecord {
            object_path: request.object_path.to_string(),
            credential_ref: request.credential_ref.redacted(),
            max_bytes: request.max_bytes,
        });
        let artifact_bytes = self
            .artifacts
            .get(request.object_path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing artifact {}", request.object_path))?;
        Ok(BackupArtifactDownloadOutcome {
            downloaded_bytes: artifact_bytes.len(),
            artifact_bytes,
            provider_metadata_is_diagnostic_only: self.metadata_is_diagnostic_only,
        })
    }
}

struct FixedKeyResolver {
    key: BackupArtifactKey,
    requests: Vec<String>,
}

impl FixedKeyResolver {
    fn new(key: BackupArtifactKey) -> Self {
        Self {
            key,
            requests: Vec::new(),
        }
    }
}

impl std::fmt::Debug for FixedKeyResolver {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FixedKeyResolver")
            .field("request_count", &self.requests.len())
            .finish()
    }
}

impl BackupArtifactKeyResolver for FixedKeyResolver {
    fn resolve_key(&mut self, key_ref: &BackupSecretRef) -> anyhow::Result<BackupArtifactKey> {
        self.requests.push(key_ref.redacted());
        Ok(self.key.clone())
    }
}

fn artifact_key() -> BackupArtifactKey {
    BackupArtifactKey::from_bytes(&[7; 32]).expect("artifact key")
}

fn protection(kind: BackupArtifactKind) -> deve_core::backup::BackupArtifactProtection {
    plan_backup_artifact_protection(BackupArtifactProtectionInput {
        artifact_kind: kind,
        key_ref: parse_backup_key_ref("keyring:deve/default-backup-key").expect("key ref"),
        encrypted: true,
        authenticated: true,
        mechanism: BackupProtectionMechanism::AeadTag,
    })
    .expect("artifact protection")
}

fn encrypted_pack_fixture(
    key: &BackupArtifactKey,
    pack_sequence: u64,
) -> (BackupBranchManifestPackRef, Vec<u8>) {
    let artifact = encrypt_backup_pack_artifact(deve_core::backup::BackupPackArtifactInput {
        repo_id: REPO_ID.parse().expect("repo id"),
        writer_identity: "writer-1",
        branch_path: "deve/branches/writer-1",
        pack_sequence,
        protection: &protection(BackupArtifactKind::Pack),
        key,
        plaintext: br#"{"ledger":["event"],"snapshots":[]}"#,
    })
    .expect("encrypted pack artifact");
    let payload_digest = artifact.payload_digest().expect("payload digest");
    let object_path = format!("deve/branches/writer-1/packs/{pack_sequence:06}.pack.enc");

    (
        BackupBranchManifestPackRef {
            pack_sequence,
            object_path,
            payload_digest,
        },
        artifact.to_bytes().expect("artifact bytes"),
    )
}

fn download_fixture() -> DownloadFixture {
    download_fixture_with_pack_count(2)
}

fn download_fixture_with_pack_count(pack_count: u64) -> DownloadFixture {
    let key = artifact_key();
    download_fixture_with_pack_key(pack_count, &key, key.clone())
}

fn download_fixture_with_pack_key(
    pack_count: u64,
    pack_key: &BackupArtifactKey,
    manifest_key: BackupArtifactKey,
) -> DownloadFixture {
    let mut pack_refs = Vec::new();
    let mut pack_artifacts = Vec::new();
    for pack_sequence in 1..=pack_count {
        let (pack_ref, pack_bytes) = encrypted_pack_fixture(pack_key, pack_sequence);
        pack_artifacts.push((pack_ref.object_path.clone(), pack_bytes));
        pack_refs.push(pack_ref);
    }
    let branch = BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap();
    let branch_manifest_path = branch.branch_manifest_path();
    let branch_manifest =
        encrypt_backup_branch_manifest_artifact(BackupBranchManifestArtifactInput {
            branch,
            repo_id: REPO_ID.parse().expect("repo id"),
            writer_identity: "writer-1",
            branch_path: "deve/branches/writer-1",
            packs: pack_refs.clone(),
            protection: &protection(BackupArtifactKind::BranchManifest),
            key: &manifest_key,
        })
        .expect("encrypted branch manifest");
    let manifest_digest = branch_manifest
        .payload_digest()
        .expect("manifest digest")
        .hex;
    let mut artifacts = vec![(
        branch_manifest_path,
        branch_manifest.to_bytes().expect("manifest bytes"),
    )];
    artifacts.extend(pack_artifacts);
    let pack_digests = pack_refs
        .into_iter()
        .map(|pack_ref| pack_ref.payload_digest.hex)
        .collect();

    (manifest_key, manifest_digest, artifacts, pack_digests)
}

fn download_input<'a>(
    manifest_digest: &'a str,
    pack_digests: &'a [String],
) -> RestoreCommandInput<'a> {
    let mut command = input(pack_digests, "remote-readonly", false, false);
    command.manifest_digest = manifest_digest;
    command.manifest_verified = false;
    command.packs_downloaded = false;
    command.packs_decrypted = false;
    command.credential_ref = Some("env:DEVE_BACKUP_CREDENTIALS");
    command.key_ref = Some("env:DEVE_BACKUP_KEY");
    command
}

fn restore_with_fixture(
    command: RestoreCommandInput<'_>,
    artifacts: ArtifactMap,
    key: BackupArtifactKey,
) -> anyhow::Result<(Vec<String>, RecordingDownloader, FixedKeyResolver)> {
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let lines = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)?;
    Ok((lines, downloader, key_resolver))
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
fn requires_download_refs_and_known_mode() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.credential_ref = None;
    let err = restore_with_fixture(command, artifacts, key)
        .expect_err("provider download requires credential metadata");
    assert!(err.to_string().contains("--credential-ref"));

    let pack_digests = vec![DIGEST_B.to_string()];
    let err =
        restore_lines(input(&pack_digests, "import", false, true)).expect_err("mode rejected");
    assert!(err.to_string().contains("mode must"));
}

#[test]
fn backup_restore_download_opens_branch_manifest_before_pack_download() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(&manifest_digest, &pack_digests);
    let (lines, downloader, key_resolver) =
        restore_with_fixture(command, artifacts, key).expect("provider download restore");

    assert_eq!(
        downloader.requests,
        vec![
            DownloadRecord {
                object_path: "deve/branches/writer-1/branch.manifest.enc".to_string(),
                credential_ref: "env:<redacted>".to_string(),
                max_bytes: BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES,
            },
            DownloadRecord {
                object_path: "deve/branches/writer-1/packs/000001.pack.enc".to_string(),
                credential_ref: "env:<redacted>".to_string(),
                max_bytes: BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES,
            },
            DownloadRecord {
                object_path: "deve/branches/writer-1/packs/000002.pack.enc".to_string(),
                credential_ref: "env:<redacted>".to_string(),
                max_bytes: BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES,
            },
        ]
    );
    assert_eq!(key_resolver.requests, vec!["env:<redacted>".to_string()]);
    assert!(lines.iter().any(|line| line == "artifact_io=true"));
    assert!(lines.iter().any(|line| line == "manifest_verified=true"));
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_flow_state=RestoreCandidate")
    );
    assert!(lines.iter().any(|line| line == "packs_decrypted=true"));
    assert!(
        lines
            .iter()
            .any(|line| line == "candidate_admission=created_remote_readonly")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_candidate_state=RemoteReadonly")
    );
}

#[test]
fn backup_restore_download_selects_pack_from_branch_manifest() {
    let (key, manifest_digest, artifacts, _pack_digests) = download_fixture();
    let empty_pack_digests = Vec::new();
    let command = download_input(&manifest_digest, &empty_pack_digests);
    let (lines, _downloader, _key_resolver) =
        restore_with_fixture(command, artifacts, key).expect("provider download restore");

    assert!(lines.iter().any(|line| line == "pack_count=2"));
    assert!(lines.iter().any(|line| line == "verified_pack_sequence=1"));
    assert!(lines.iter().any(|line| line == "verified_pack_sequence=2"));
    assert!(lines.iter().any(|line| {
        line == "verified_pack_object_path=deve/branches/writer-1/packs/000001.pack.enc"
    }));
    assert!(lines.iter().any(|line| {
        line == "verified_pack_object_path=deve/branches/writer-1/packs/000002.pack.enc"
    }));
}

#[test]
fn backup_restore_download_verifies_branch_manifest_digest_and_routing() {
    let (key, _manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(DIGEST_A, &pack_digests);
    let err = restore_with_fixture(command, artifacts, key)
        .expect_err("manifest digest mismatch must fail closed");

    assert!(err.to_string().contains("branch manifest artifact digest"));
}

#[test]
fn backup_restore_download_rejects_manual_decrypt_evidence_before_provider_get() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.packs_decrypted = true;
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("download branch must reject precomputed decrypted evidence");

    assert!(err.to_string().contains("precomputed evidence"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_download_opens_pack_artifacts_from_branch_manifest_refs() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(&manifest_digest, &pack_digests);
    let (lines, downloader, _key_resolver) =
        restore_with_fixture(command, artifacts, key).expect("provider download restore");

    assert_eq!(downloader.requests.len(), 3);
    assert!(lines.iter().any(|line| line == "packs_decrypted=true"));
    assert!(lines.iter().any(|line| line == "pack_count=2"));
    assert!(
        lines
            .iter()
            .any(|line| line == "candidate_admission=created_remote_readonly")
    );
}

#[test]
fn backup_restore_download_admits_remote_readonly_candidate_after_pack_decrypt() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(&manifest_digest, &pack_digests);
    let (lines, _downloader, _key_resolver) =
        restore_with_fixture(command, artifacts, key).expect("provider download restore");

    assert!(
        lines
            .iter()
            .any(|line| line == "restore_flow_state=RestoreCandidate")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "restore_candidate_state=RemoteReadonly")
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "writes_local_authority=false")
    );
}

#[test]
fn backup_restore_download_rejects_wrong_key_before_candidate() {
    let wrong_pack_key = BackupArtifactKey::from_bytes(&[8; 32]).expect("wrong pack key");
    let (manifest_key, manifest_digest, artifacts, pack_digests) =
        download_fixture_with_pack_key(2, &wrong_pack_key, artifact_key());
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(manifest_key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("wrong key must fail before candidate admission");

    assert!(err.to_string().contains("decryption failed"));
    assert_eq!(key_resolver.requests, vec!["env:<redacted>".to_string()]);
    assert_eq!(downloader.requests.len(), 3);
}

#[test]
fn backup_restore_download_rejects_resource_budget_excess() {
    let pack_count = BACKUP_RESTORE_MAX_PACKS + 1;
    let (key, manifest_digest, artifacts, pack_digests) =
        download_fixture_with_pack_count(pack_count);
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("resource budget excess must fail closed before pack download");

    assert!(err.to_string().contains("resource budget"));
    assert_eq!(downloader.requests.len(), 1);
}

#[test]
fn backup_restore_download_rejects_manual_evidence_before_provider_get() {
    let forbidden_flags: [ForbiddenFlagCase; 3] = [
        ("manifest", |command| command.manifest_verified = true),
        ("downloaded", |command| command.packs_downloaded = true),
        ("decrypted", |command| command.packs_decrypted = true),
    ];

    for (label, apply_flag) in forbidden_flags {
        let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
        let mut command = download_input(&manifest_digest, &pack_digests);
        apply_flag(&mut command);
        let mut downloader = RecordingDownloader::new(artifacts);
        let mut key_resolver = FixedKeyResolver::new(key);
        let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
            .expect_err(&format!("manual {label} evidence must fail closed"));

        assert!(err.to_string().contains("precomputed evidence"));
        assert!(downloader.requests.is_empty());
    }
}

#[test]
fn backup_restore_download_rejects_manual_pack_metadata_before_provider_get() {
    let forbidden_flags: [ForbiddenFlagCase; 5] = [
        ("pack_sequence", |command| command.pack_sequence = Some(1)),
        ("ledger_start", |command| command.ledger_start = Some(1)),
        ("ledger_end", |command| command.ledger_end = Some(1)),
        ("ledger_events", |command| {
            command.ledger_event_count = Some(1)
        }),
        ("snapshot_count", |command| command.snapshot_count = Some(0)),
    ];

    for (label, apply_flag) in forbidden_flags {
        let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
        let mut command = download_input(&manifest_digest, &pack_digests);
        apply_flag(&mut command);
        let mut downloader = RecordingDownloader::new(artifacts);
        let mut key_resolver = FixedKeyResolver::new(key);
        let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
            .expect_err(&format!("manual {label} metadata must fail closed"));

        assert!(err.to_string().contains("branch.manifest.enc"));
        assert!(downloader.requests.is_empty());
    }
}

#[test]
fn backup_restore_download_rejects_tampered_artifact_before_candidate() {
    let (key, manifest_digest, mut artifacts, pack_digests) = download_fixture();
    for (path, bytes) in &mut artifacts {
        if path.ends_with("000001.pack.enc") {
            bytes.push(b'\n');
        }
    }
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("tampered artifact must fail before candidate admission");

    assert!(err.to_string().contains("digest"));
    assert_eq!(downloader.requests.len(), 2);
}

#[test]
fn backup_restore_download_rejects_authoritative_provider_metadata() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input(&manifest_digest, &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts).with_authoritative_metadata();
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("provider metadata must remain diagnostic only");

    assert!(err.to_string().contains("diagnostic-only"));
    assert_eq!(downloader.requests.len(), 1);
}

#[test]
fn backup_restore_download_rejects_metadata_before_provider_get() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.manifest_repo_id = "22222222-2222-2222-2222-222222222222";
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("manifest mismatch must fail before provider I/O");

    assert!(err.to_string().contains("repo id"));
    assert!(downloader.requests.is_empty());

    let (key, _manifest_digest, artifacts, pack_digests) = download_fixture();
    let command = download_input("not-a-sha256-digest", &pack_digests);
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("invalid manifest digest must fail before provider I/O");

    assert!(err.to_string().contains("manifest-digest"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_explicit_import_non_dry_run_remains_fail_closed() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.mode = "explicit-import";
    command.write_gate = true;
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("explicit import execution must remain closed");

    assert!(err.to_string().contains("fail-closed"));
    assert!(downloader.requests.is_empty());
}

#[test]
fn backup_restore_explicit_merge_non_dry_run_remains_fail_closed() {
    let (key, manifest_digest, artifacts, pack_digests) = download_fixture();
    let mut command = download_input(&manifest_digest, &pack_digests);
    command.mode = "explicit-merge";
    command.write_gate = true;
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let err = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)
        .expect_err("explicit merge execution must remain closed");

    assert!(err.to_string().contains("fail-closed"));
    assert!(downloader.requests.is_empty());
}
