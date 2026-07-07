use super::command_input::REPO_ID;
use deve_core::backup::{
    BACKUP_PACK_PLAINTEXT_FORMAT_VERSION, BackupArtifactKey, BackupArtifactKind,
    BackupArtifactProtection, BackupArtifactProtectionInput, BackupBlobRef,
    BackupBranchManifestArtifactInput, BackupBranchManifestPackRef, BackupDigest, BackupLocator,
    BackupPackManifest, BackupPackPlaintext, BackupPackPlaintextEncodeInput,
    BackupPackPlaintextLedgerEntry, BackupPackPlanInput, BackupProtectionMechanism, BackupSeqRange,
    encode_backup_pack_plaintext, encrypt_backup_branch_manifest_artifact,
    encrypt_backup_pack_artifact, parse_backup_key_ref, plan_backup_artifact_protection,
    plan_backup_pack,
};
use deve_core::models::{ContentOp, DocId, LedgerEntry, PeerId, serialize_ledger_entry};

pub(in crate::commands::backup::restore::tests) type ArtifactMap = Vec<(String, Vec<u8>)>;
pub(in crate::commands::backup::restore::tests) type DownloadFixture =
    (BackupArtifactKey, String, ArtifactMap, Vec<String>);

pub(in crate::commands::backup::restore::tests) fn artifact_key() -> BackupArtifactKey {
    BackupArtifactKey::from_bytes(&[7; 32]).expect("artifact key")
}

pub(in crate::commands::backup::restore::tests) fn protection(
    kind: BackupArtifactKind,
) -> BackupArtifactProtection {
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

pub(in crate::commands::backup::restore::tests) fn encrypted_pack_fixture_with_plaintext(
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

pub(in crate::commands::backup::restore::tests) fn download_fixture() -> DownloadFixture {
    download_fixture_with_pack_count(2)
}

pub(in crate::commands::backup::restore::tests) fn download_fixture_with_pack_count(
    pack_count: u64,
) -> DownloadFixture {
    let key = artifact_key();
    download_fixture_with_pack_key(pack_count, &key, key.clone())
}

pub(in crate::commands::backup::restore::tests) fn download_fixture_with_pack_key(
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
