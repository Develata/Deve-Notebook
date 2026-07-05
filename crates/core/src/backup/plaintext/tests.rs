use super::*;
use crate::backup::{BackupDigest, BackupPackPlanInput, plan_backup_pack};
use crate::models::{ContentOp, DocId, LedgerEntry, PeerId, serialize_ledger_entry};

fn digest() -> BackupDigest {
    BackupDigest::sha256("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
}

fn blob_ref(path: &str) -> BackupBlobRef {
    BackupBlobRef {
        path: path.into(),
        size_bytes: 12,
        digest: digest(),
    }
}

fn manifest() -> BackupPackManifest {
    plan_backup_pack(BackupPackPlanInput {
        repo_id: uuid::Uuid::from_u128(77),
        writer_identity: "writer-1".into(),
        branch_path: "deve/branches/writer-1".into(),
        pack_sequence: 3,
        ledger_seq_range: Some(BackupSeqRange { start: 10, end: 11 }),
        ledger_event_count: 2,
        snapshot_count: 1,
        payload_digest: digest(),
        blob_refs: vec![blob_ref("blobs/aa.bin")],
    })
    .unwrap()
}

fn ledger_entry(global_seq: u64) -> BackupPackPlaintextLedgerEntry {
    let entry = LedgerEntry::new_content(
        DocId::from_u128(900 + u128::from(global_seq)),
        ContentOp::Insert {
            pos: 0,
            content: format!("entry-{global_seq}").into(),
        },
        1_700_000_000 + i64::try_from(global_seq).unwrap(),
        PeerId::new("backup-test-peer"),
        global_seq,
        None,
        None,
    );

    BackupPackPlaintextLedgerEntry {
        global_seq,
        entry_bytes: serialize_ledger_entry(&entry).unwrap(),
    }
}

fn plaintext_for(manifest: &BackupPackManifest) -> BackupPackPlaintext {
    BackupPackPlaintext {
        format_version: BACKUP_PACK_PLAINTEXT_FORMAT_VERSION,
        repo_id: manifest.repo_id,
        writer_identity: manifest.writer_identity.clone(),
        branch_path: manifest.branch_path.clone(),
        pack_sequence: manifest.pack_sequence,
        ledger_seq_range: manifest.ledger_seq_range,
        ledger_entries: vec![ledger_entry(10), ledger_entry(11)],
        snapshot_refs: vec![blob_ref("snapshots/000001.bin")],
        blob_refs: manifest.blob_refs.clone(),
    }
}

fn raw_plaintext_bytes(plaintext: &BackupPackPlaintext) -> Vec<u8> {
    let payload = bincode::serialize(plaintext).unwrap();
    let mut bytes = Vec::with_capacity(BACKUP_PACK_PLAINTEXT_MAGIC.len() + payload.len());
    bytes.extend_from_slice(BACKUP_PACK_PLAINTEXT_MAGIC);
    bytes.extend(payload);
    bytes
}

#[test]
fn backup_pack_plaintext_roundtrips_manifest_aligned_ledger_entries() {
    let manifest = manifest();
    let plaintext = plaintext_for(&manifest);

    let encoded = encode_backup_pack_plaintext(BackupPackPlaintextEncodeInput {
        manifest: &manifest,
        plaintext: &plaintext,
    })
    .unwrap();
    let opened = open_backup_pack_plaintext(BackupPackPlaintextOpenInput {
        manifest: &manifest,
        plaintext_bytes: &encoded,
    })
    .unwrap();

    assert_eq!(opened.repo_id, manifest.repo_id);
    assert_eq!(opened.ledger_entries.len(), 2);
    assert_eq!(opened.decoded_ledger_entries().unwrap().len(), 2);
    assert_eq!(opened.blob_refs, manifest.blob_refs);
}

#[test]
fn backup_pack_plaintext_rejects_manifest_mismatch() {
    let manifest = manifest();
    let mut plaintext = plaintext_for(&manifest);
    plaintext.repo_id = uuid::Uuid::from_u128(78);

    assert_eq!(
        encode_backup_pack_plaintext(BackupPackPlaintextEncodeInput {
            manifest: &manifest,
            plaintext: &plaintext,
        })
        .expect_err("repo mismatch must fail"),
        BackupPackPlaintextError::RepoIdMismatch
    );

    let mut plaintext = plaintext_for(&manifest);
    plaintext.blob_refs.clear();
    let raw = raw_plaintext_bytes(&plaintext);
    assert_eq!(
        open_backup_pack_plaintext(BackupPackPlaintextOpenInput {
            manifest: &manifest,
            plaintext_bytes: &raw,
        })
        .expect_err("blob ref mismatch must fail"),
        BackupPackPlaintextError::BlobRefsMismatch
    );
}

#[test]
fn backup_pack_plaintext_rejects_unversioned_or_invalid_ledger_entries() {
    let manifest = manifest();
    let plaintext = plaintext_for(&manifest);
    let unversioned = bincode::serialize(&plaintext).unwrap();
    assert_eq!(
        open_backup_pack_plaintext(BackupPackPlaintextOpenInput {
            manifest: &manifest,
            plaintext_bytes: &unversioned,
        })
        .expect_err("missing magic must fail"),
        BackupPackPlaintextError::MissingMagic
    );

    let mut plaintext = plaintext_for(&manifest);
    plaintext.ledger_entries[0].entry_bytes = b"not-a-versioned-ledger-entry".to_vec();
    let raw = raw_plaintext_bytes(&plaintext);
    assert_eq!(
        open_backup_pack_plaintext(BackupPackPlaintextOpenInput {
            manifest: &manifest,
            plaintext_bytes: &raw,
        })
        .expect_err("invalid ledger bytes must fail"),
        BackupPackPlaintextError::InvalidLedgerEntry
    );
}

#[test]
fn backup_pack_plaintext_rejects_non_contiguous_ledger_sequences() {
    let manifest = manifest();
    let mut plaintext = plaintext_for(&manifest);
    plaintext.ledger_entries[1] = ledger_entry(12);
    let raw = raw_plaintext_bytes(&plaintext);

    assert_eq!(
        open_backup_pack_plaintext(BackupPackPlaintextOpenInput {
            manifest: &manifest,
            plaintext_bytes: &raw,
        })
        .expect_err("ledger global seq must match manifest range"),
        BackupPackPlaintextError::LedgerSequenceMismatch
    );
}

#[test]
fn backup_pack_plaintext_writes_no_local_authority() {
    let scratch = tempfile::tempdir().unwrap();
    let authority_paths = [
        scratch.path().join("ledger"),
        scratch.path().join("staging"),
        scratch.path().join("projection-workspace"),
        scratch.path().join(".git"),
        scratch.path().join(".notegit"),
    ];
    let manifest = manifest();
    let plaintext = plaintext_for(&manifest);

    let encoded = encode_backup_pack_plaintext(BackupPackPlaintextEncodeInput {
        manifest: &manifest,
        plaintext: &plaintext,
    })
    .unwrap();
    open_backup_pack_plaintext(BackupPackPlaintextOpenInput {
        manifest: &manifest,
        plaintext_bytes: &encoded,
    })
    .unwrap();

    for path in authority_paths {
        assert!(
            !path.exists(),
            "plaintext schema gate must not create local authority path {}",
            path.display()
        );
    }
}

#[test]
fn backup_pack_plaintext_rejects_invalid_snapshot_refs() {
    let manifest = manifest();
    let mut plaintext = plaintext_for(&manifest);
    plaintext.snapshot_refs[0].path = "snapshots\\bad.bin".into();
    let raw = raw_plaintext_bytes(&plaintext);

    assert_eq!(
        open_backup_pack_plaintext(BackupPackPlaintextOpenInput {
            manifest: &manifest,
            plaintext_bytes: &raw,
        })
        .expect_err("non-canonical snapshot ref must fail"),
        BackupPackPlaintextError::InvalidBlobRef
    );
}
