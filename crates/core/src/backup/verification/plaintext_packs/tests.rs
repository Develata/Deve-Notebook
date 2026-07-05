use super::*;
use crate::backup::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BACKUP_PACK_PLAINTEXT_FORMAT_VERSION, BackupArtifactKey,
    BackupArtifactKind, BackupArtifactProtection, BackupArtifactProtectionInput, BackupBlobRef,
    BackupBranchManifest, BackupBranchManifestPackRef, BackupEncryptedPackArtifact,
    BackupPackArtifactDownloadVerifyInput, BackupPackArtifactInput, BackupPackArtifactOpenInput,
    BackupPackArtifactOpenResult, BackupPackManifest, BackupPackPlaintext,
    BackupPackPlaintextEncodeInput, BackupPackPlaintextLedgerEntry, BackupPackPlanInput,
    BackupProtectionMechanism, BackupSeqRange, encode_backup_pack_plaintext,
    encrypt_backup_pack_artifact, open_backup_pack_artifact, parse_backup_key_ref,
    plan_backup_artifact_protection, plan_backup_pack, verify_decrypted_backup_packs,
    verify_downloaded_backup_packs, verify_downloaded_pack_artifact_digest_and_routing,
};
use crate::models::{ContentOp, DocId, LedgerEntry, PeerId, serialize_ledger_entry};

struct PackFixture {
    pack_manifest: BackupPackManifest,
    download_result: crate::backup::BackupPackArtifactDownloadVerifyResult,
    open_result: BackupPackArtifactOpenResult,
}

fn repo_id() -> RepoId {
    uuid::Uuid::from_u128(31)
}

fn digest(fill: char) -> BackupDigest {
    BackupDigest::sha256(fill.to_string().repeat(64))
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

fn ledger_entry(global_seq: u64) -> BackupPackPlaintextLedgerEntry {
    let entry = LedgerEntry::new_content(
        DocId::from_u128(1000 + u128::from(global_seq)),
        ContentOp::Insert {
            pos: 0,
            content: format!("restore-entry-{global_seq}").into(),
        },
        1_700_000_000 + i64::try_from(global_seq).unwrap(),
        PeerId::new("backup-plaintext-test-peer"),
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

fn pack_fixture(pack_sequence: u64, writer_identity: &str) -> PackFixture {
    let branch_path = format!("deve/branches/{writer_identity}");
    let provisional_manifest =
        pack_manifest(pack_sequence, writer_identity, &branch_path, digest('a'));
    let plaintext = plaintext_bytes(&provisional_manifest);
    pack_fixture_with_plaintext(pack_sequence, writer_identity, &branch_path, &plaintext)
}

fn pack_fixture_with_plaintext(
    pack_sequence: u64,
    writer_identity: &str,
    branch_path: &str,
    plaintext: &[u8],
) -> PackFixture {
    let key = artifact_key();
    let protection = protection();
    let artifact = encrypt_backup_pack_artifact(pack_artifact_input(
        &key,
        &protection,
        pack_sequence,
        writer_identity,
        branch_path,
        plaintext,
    ))
    .unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let pack_manifest = pack_manifest_for_artifact(&artifact);
    let download_result =
        verify_downloaded_pack_artifact_digest_and_routing(BackupPackArtifactDownloadVerifyInput {
            manifest: &pack_manifest,
            artifact_bytes: &artifact_bytes,
        })
        .unwrap();
    let open_result = open_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &pack_manifest,
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .unwrap();

    PackFixture {
        pack_manifest,
        download_result,
        open_result,
    }
}

fn pack_manifest_for_artifact(artifact: &BackupEncryptedPackArtifact) -> BackupPackManifest {
    pack_manifest(
        artifact.pack_sequence,
        &artifact.writer_identity,
        &artifact.branch_path,
        artifact.payload_digest().unwrap(),
    )
}

fn fixtures() -> Vec<PackFixture> {
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

fn decrypted_result(
    manifest: &BackupBranchManifest,
    fixtures: Vec<PackFixture>,
) -> BackupDecryptedPacksResult {
    let downloaded = verify_downloaded_backup_packs(crate::backup::BackupDownloadedPacksInput {
        branch_manifest: manifest,
        verified_packs: fixtures
            .iter()
            .map(|fixture| fixture.download_result.clone())
            .collect(),
    })
    .unwrap();
    verify_decrypted_backup_packs(crate::backup::BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: fixtures
            .into_iter()
            .map(|fixture| fixture.open_result)
            .collect(),
    })
    .unwrap()
}

#[test]
fn backup_plaintext_packs_open_schema_from_verified_branch_manifest_refs() {
    let fixtures = fixtures();
    let manifest = branch_manifest(&fixtures);
    let decrypted = decrypted_result(&manifest, fixtures);

    let result = verify_plaintext_backup_packs(BackupPlaintextPacksInput {
        branch_manifest: &manifest,
        decrypted_packs: &decrypted,
    })
    .expect("plaintext schema evidence");

    assert_eq!(result.repo_id(), repo_id());
    assert_eq!(result.writer_identity(), "writer-1");
    assert_eq!(result.branch_path(), "deve/branches/writer-1");
    assert_eq!(result.pack_count(), 2);
    assert_eq!(result.pack_digests(), decrypted.pack_digests());
    assert_eq!(result.plaintext_packs()[0].plaintext().pack_sequence, 1);
    assert_eq!(result.plaintext_packs()[1].plaintext().pack_sequence, 2);
}

#[test]
fn backup_plaintext_packs_reject_raw_or_mismatched_plaintext() {
    let mut raw_fixtures = fixtures();
    raw_fixtures[0] = pack_fixture_with_plaintext(
        1,
        "writer-1",
        "deve/branches/writer-1",
        b"raw plaintext without schema",
    );
    let manifest = branch_manifest(&raw_fixtures);
    let decrypted = decrypted_result(&manifest, raw_fixtures);

    let err = verify_plaintext_backup_packs(BackupPlaintextPacksInput {
        branch_manifest: &manifest,
        decrypted_packs: &decrypted,
    })
    .expect_err("raw plaintext must not enter restore candidate");

    assert!(matches!(err, BackupVerificationError::PackPlaintext(_)));

    let fixtures = fixtures();
    let mut manifest = branch_manifest(&fixtures);
    manifest.packs[0].blob_refs[0].path = "blobs/mismatched.bin".into();
    let decrypted = decrypted_result(&manifest, fixtures);

    let err = verify_plaintext_backup_packs(BackupPlaintextPacksInput {
        branch_manifest: &manifest,
        decrypted_packs: &decrypted,
    })
    .expect_err("branch manifest metadata mismatch must not self-certify");

    assert!(matches!(err, BackupVerificationError::PackPlaintext(_)));
}
