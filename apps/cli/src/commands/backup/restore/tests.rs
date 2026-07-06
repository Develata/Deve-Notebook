use super::{RestoreCommandInput, restore_lines, restore_lines_with_runtime};
use crate::commands::backup::provider_io::{
    BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES, BackupArtifactDownloadOutcome,
    BackupArtifactDownloadRequest, BackupArtifactDownloader, BackupArtifactKeyResolver,
};
use deve_core::backup::{
    BACKUP_PACK_PLAINTEXT_FORMAT_VERSION, BACKUP_RESTORE_MAX_PACKS, BackupArtifactKey,
    BackupArtifactKind, BackupArtifactProtectionInput, BackupBlobRef,
    BackupBranchManifestArtifactInput, BackupBranchManifestPackRef, BackupDigest, BackupLocator,
    BackupPackManifest, BackupPackPlaintext, BackupPackPlaintextEncodeInput,
    BackupPackPlaintextLedgerEntry, BackupPackPlanInput, BackupProtectionMechanism,
    BackupSecretRef, BackupSeqRange, encode_backup_pack_plaintext,
    encrypt_backup_branch_manifest_artifact, encrypt_backup_pack_artifact, parse_backup_key_ref,
    plan_backup_artifact_protection, plan_backup_pack,
};
use deve_core::models::{ContentOp, DocId, LedgerEntry, PeerId, serialize_ledger_entry};
use std::collections::HashMap;

mod download;

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
    reported_downloaded_bytes: Option<usize>,
    requests: Vec<DownloadRecord>,
}

impl RecordingDownloader {
    fn new(artifacts: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
        Self {
            artifacts: artifacts.into_iter().collect(),
            metadata_is_diagnostic_only: true,
            reported_downloaded_bytes: None,
            requests: Vec::new(),
        }
    }

    fn with_authoritative_metadata(mut self) -> Self {
        self.metadata_is_diagnostic_only = false;
        self
    }

    fn with_reported_downloaded_bytes(mut self, downloaded_bytes: usize) -> Self {
        self.reported_downloaded_bytes = Some(downloaded_bytes);
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
        let downloaded_bytes = self
            .reported_downloaded_bytes
            .unwrap_or(artifact_bytes.len());
        Ok(BackupArtifactDownloadOutcome {
            downloaded_bytes,
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

fn digest(fill: char) -> BackupDigest {
    BackupDigest::sha256(fill.to_string().repeat(64))
}

fn blob_ref(pack_sequence: u64) -> BackupBlobRef {
    BackupBlobRef {
        path: format!("blobs/{pack_sequence:06}.bin"),
        size_bytes: 12,
        digest: digest('b'),
    }
}

fn snapshot_ref(pack_sequence: u64) -> BackupBlobRef {
    BackupBlobRef {
        path: format!("snapshots/{pack_sequence:06}.bin"),
        size_bytes: 24,
        digest: digest('c'),
    }
}

fn pack_manifest(pack_sequence: u64, payload_digest: BackupDigest) -> BackupPackManifest {
    plan_backup_pack(BackupPackPlanInput {
        repo_id: REPO_ID.parse().expect("repo id"),
        writer_identity: "writer-1".to_string(),
        branch_path: "deve/branches/writer-1".to_string(),
        pack_sequence,
        ledger_seq_range: Some(BackupSeqRange {
            start: pack_sequence,
            end: pack_sequence,
        }),
        ledger_event_count: 1,
        snapshot_count: 1,
        payload_digest,
        blob_refs: vec![blob_ref(pack_sequence)],
    })
    .expect("pack manifest")
}

fn ledger_entry(global_seq: u64) -> BackupPackPlaintextLedgerEntry {
    let entry = LedgerEntry::new_content(
        DocId::from_u128(10_000 + u128::from(global_seq)),
        ContentOp::Insert {
            pos: 0,
            content: format!("restore-entry-{global_seq}").into(),
        },
        1_700_000_000 + i64::try_from(global_seq).expect("timestamp seq"),
        PeerId::new("backup-cli-restore-test-peer"),
        global_seq,
        None,
        None,
    );
    BackupPackPlaintextLedgerEntry {
        global_seq,
        entry_bytes: serialize_ledger_entry(&entry).expect("versioned ledger entry"),
    }
}

fn plaintext_bytes(manifest: &BackupPackManifest) -> Vec<u8> {
    let seq_range = manifest.ledger_seq_range.expect("ledger range");
    let plaintext = BackupPackPlaintext {
        format_version: BACKUP_PACK_PLAINTEXT_FORMAT_VERSION,
        repo_id: manifest.repo_id,
        writer_identity: manifest.writer_identity.clone(),
        branch_path: manifest.branch_path.clone(),
        pack_sequence: manifest.pack_sequence,
        ledger_seq_range: manifest.ledger_seq_range,
        ledger_entries: vec![ledger_entry(seq_range.start)],
        snapshot_refs: vec![snapshot_ref(manifest.pack_sequence)],
        blob_refs: manifest.blob_refs.clone(),
    };
    encode_backup_pack_plaintext(BackupPackPlaintextEncodeInput {
        manifest,
        plaintext: &plaintext,
    })
    .expect("backup pack plaintext")
}

fn encrypted_pack_fixture(
    key: &BackupArtifactKey,
    pack_sequence: u64,
) -> (BackupBranchManifestPackRef, Vec<u8>) {
    let provisional_manifest = pack_manifest(pack_sequence, digest('a'));
    let plaintext = plaintext_bytes(&provisional_manifest);
    encrypted_pack_fixture_with_plaintext(key, pack_sequence, &plaintext)
}

fn encrypted_pack_fixture_with_plaintext(
    key: &BackupArtifactKey,
    pack_sequence: u64,
    plaintext: &[u8],
) -> (BackupBranchManifestPackRef, Vec<u8>) {
    let artifact = encrypt_backup_pack_artifact(deve_core::backup::BackupPackArtifactInput {
        repo_id: REPO_ID.parse().expect("repo id"),
        writer_identity: "writer-1",
        branch_path: "deve/branches/writer-1",
        pack_sequence,
        protection: &protection(BackupArtifactKind::Pack),
        key,
        plaintext,
    })
    .expect("encrypted pack artifact");
    let payload_digest = artifact.payload_digest().expect("payload digest");
    let manifest = pack_manifest(pack_sequence, payload_digest);

    (
        BackupBranchManifestPackRef::from_pack_manifest(&manifest),
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
        |line| line == "candidate_admission=typed_verification_and_plaintext_evidence_required"
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
