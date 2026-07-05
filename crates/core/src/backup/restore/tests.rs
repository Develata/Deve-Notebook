mod admission;
mod budget;
mod rejection;

use super::{BackupDigest, RestoreAdmissionMode, RestoreCandidateInput, RestoreEvidence};
use crate::backup::BackupLocator;
use crate::backup::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BACKUP_PACK_PLAINTEXT_FORMAT_VERSION, BackupArtifactKey,
    BackupArtifactKind, BackupArtifactProtection, BackupArtifactProtectionInput, BackupBlobRef,
    BackupBranchManifest, BackupBranchManifestPackRef, BackupEncryptedPackArtifact,
    BackupPackArtifactDownloadVerifyInput, BackupPackArtifactInput, BackupPackArtifactOpenInput,
    BackupPackArtifactOpenResult, BackupPackManifest, BackupPackPlaintext,
    BackupPackPlaintextEncodeInput, BackupPackPlaintextLedgerEntry, BackupPackPlanInput,
    BackupPackVerificationEvidence, BackupPlaintextPacksInput, BackupPlaintextPacksResult,
    BackupProtectionMechanism, BackupSeqRange, BackupVerificationInput, BackupVerificationResult,
    encode_backup_pack_plaintext, encrypt_backup_pack_artifact, open_backup_pack_artifact,
    parse_backup_key_ref, plan_backup_artifact_protection, plan_backup_pack,
    verify_backup_artifacts, verify_decrypted_backup_packs, verify_downloaded_backup_packs,
    verify_downloaded_pack_artifact_digest_and_routing, verify_plaintext_backup_packs,
};
use crate::models::{ContentOp, DocId, LedgerEntry, PeerId, RepoId, serialize_ledger_entry};

struct PackFixture {
    pack_manifest: BackupPackManifest,
    download_result: crate::backup::BackupPackArtifactDownloadVerifyResult,
    open_result: BackupPackArtifactOpenResult,
}

struct RestoreEvidenceFixture {
    manifest_verification: BackupVerificationResult,
    plaintext_packs: BackupPlaintextPacksResult,
}

fn repo_id() -> RepoId {
    uuid::Uuid::from_u128(42)
}

fn digest(seed: char) -> BackupDigest {
    BackupDigest::sha256(seed.to_string().repeat(64))
}

fn uppercase_digest(seed: char) -> BackupDigest {
    BackupDigest::sha256(seed.to_ascii_uppercase().to_string().repeat(64))
}

fn numbered_digest(index: u64) -> BackupDigest {
    BackupDigest::sha256(format!("{index:064x}"))
}

fn artifact_key() -> BackupArtifactKey {
    BackupArtifactKey::from_bytes(&[9; 32]).unwrap()
}

fn protection() -> BackupArtifactProtection {
    plan_backup_artifact_protection(BackupArtifactProtectionInput {
        artifact_kind: BackupArtifactKind::Pack,
        key_ref: parse_backup_key_ref("keyring:deve/default-backup-key").unwrap(),
        encrypted: true,
        authenticated: true,
        mechanism: BackupProtectionMechanism::AeadTag,
    })
    .unwrap()
}

fn pack_artifact_input<'a>(
    key: &'a BackupArtifactKey,
    protection: &'a BackupArtifactProtection,
    pack_sequence: u64,
    writer_identity: &'a str,
    branch_path: &'a str,
    plaintext: &'a [u8],
) -> BackupPackArtifactInput<'a> {
    BackupPackArtifactInput {
        repo_id: repo_id(),
        writer_identity,
        branch_path,
        pack_sequence,
        protection,
        key,
        plaintext,
    }
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

fn pack_manifest(
    pack_sequence: u64,
    writer_identity: &str,
    branch_path: &str,
    payload_digest: BackupDigest,
) -> BackupPackManifest {
    plan_backup_pack(BackupPackPlanInput {
        repo_id: repo_id(),
        writer_identity: writer_identity.to_owned(),
        branch_path: branch_path.to_owned(),
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
    .unwrap()
}

fn pack_manifest_for(artifact: &BackupEncryptedPackArtifact) -> BackupPackManifest {
    pack_manifest(
        artifact.pack_sequence,
        &artifact.writer_identity,
        &artifact.branch_path,
        artifact.payload_digest().unwrap(),
    )
}

fn ledger_entry(global_seq: u64) -> BackupPackPlaintextLedgerEntry {
    let entry = LedgerEntry::new_content(
        DocId::from_u128(1000 + u128::from(global_seq)),
        ContentOp::Insert {
            pos: 0,
            content: format!("restore-entry-{global_seq}").into(),
        },
        1_700_000_000 + i64::try_from(global_seq).unwrap(),
        PeerId::new("backup-restore-test-peer"),
        global_seq,
        None,
        None,
    );
    BackupPackPlaintextLedgerEntry {
        global_seq,
        entry_bytes: serialize_ledger_entry(&entry).unwrap(),
    }
}

fn plaintext_bytes(manifest: &BackupPackManifest) -> Vec<u8> {
    let seq_range = manifest.ledger_seq_range.unwrap();
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
    .unwrap()
}

fn pack_fixture(pack_sequence: u64, writer_identity: &str) -> PackFixture {
    let key = artifact_key();
    let protection = protection();
    let branch_path = format!("deve/branches/{writer_identity}");
    let provisional_manifest =
        pack_manifest(pack_sequence, writer_identity, &branch_path, digest('a'));
    let plaintext = plaintext_bytes(&provisional_manifest);
    let artifact = encrypt_backup_pack_artifact(pack_artifact_input(
        &key,
        &protection,
        pack_sequence,
        writer_identity,
        &branch_path,
        &plaintext,
    ))
    .unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let manifest = pack_manifest_for(&artifact);
    let download_result =
        verify_downloaded_pack_artifact_digest_and_routing(BackupPackArtifactDownloadVerifyInput {
            manifest: &manifest,
            artifact_bytes: &artifact_bytes,
        })
        .unwrap();
    let open_result = open_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &manifest,
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .unwrap();

    PackFixture {
        pack_manifest: manifest,
        download_result,
        open_result,
    }
}

fn pack_fixtures() -> Vec<PackFixture> {
    vec![pack_fixture(1, "writer-1"), pack_fixture(2, "writer-1")]
}

fn branch_manifest(fixtures: &[PackFixture]) -> BackupBranchManifest {
    BackupBranchManifest {
        repo_id: repo_id(),
        writer_identity: "writer-1".into(),
        branch_path: "deve/branches/writer-1".into(),
        branch_manifest_path: "deve/branches/writer-1/branch.manifest.enc".into(),
        pack_prefix: "deve/branches/writer-1/packs".into(),
        format_version: BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
        packs: fixtures
            .iter()
            .map(|fixture| BackupBranchManifestPackRef::from_pack_manifest(&fixture.pack_manifest))
            .collect(),
    }
}

fn manifest_verification(pack_digests: Vec<BackupDigest>) -> BackupVerificationResult {
    let packs = pack_digests
        .into_iter()
        .enumerate()
        .map(|(index, digest)| (u64::try_from(index + 1).unwrap(), digest))
        .collect::<Vec<_>>();
    manifest_verification_with_sequences(packs)
}

fn manifest_verification_with_sequences(
    pack_refs: Vec<(u64, BackupDigest)>,
) -> BackupVerificationResult {
    let packs = pack_refs
        .into_iter()
        .map(|(pack_sequence, digest)| BackupPackVerificationEvidence {
            pack_sequence,
            expected_digest: digest.clone(),
            computed_digest: digest,
            authenticated: true,
            decrypted: true,
        })
        .collect();
    verify_backup_artifacts(BackupVerificationInput {
        expected_repo_id: repo_id(),
        manifest_repo_id: repo_id(),
        expected_manifest_digest: digest('a'),
        computed_manifest_digest: digest('a'),
        manifest_authenticated: true,
        packs,
        decrypt_required: true,
    })
    .unwrap()
}

fn verified_restore_evidence() -> RestoreEvidenceFixture {
    let fixtures = pack_fixtures();
    let manifest = branch_manifest(&fixtures);
    let downloaded = verify_downloaded_backup_packs(crate::backup::BackupDownloadedPacksInput {
        branch_manifest: &manifest,
        verified_packs: fixtures
            .iter()
            .map(|fixture| fixture.download_result.clone())
            .collect(),
    })
    .unwrap();
    let manifest_verification = manifest_verification(downloaded.pack_digests().to_vec());
    let decrypted_packs = verify_decrypted_backup_packs(crate::backup::BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: fixtures
            .into_iter()
            .map(|fixture| fixture.open_result)
            .collect(),
    })
    .unwrap();
    let plaintext_packs = verify_plaintext_backup_packs(BackupPlaintextPacksInput {
        branch_manifest: &manifest,
        decrypted_packs: &decrypted_packs,
    })
    .unwrap();

    RestoreEvidenceFixture {
        manifest_verification,
        plaintext_packs,
    }
}

fn input() -> RestoreCandidateInput {
    let branch = BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap();

    RestoreCandidateInput {
        repo_id: repo_id(),
        expected_repo_id: repo_id(),
        writer_identity: branch.writer_identity,
        branch_path: format!("{}/", branch.branch_path),
        manifest_digest: digest('a'),
        pack_count: 2,
        pack_digests: vec![digest('b'), digest('c')],
        evidence: RestoreEvidence::verified_downloaded_decrypted_plaintext(),
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    }
}
