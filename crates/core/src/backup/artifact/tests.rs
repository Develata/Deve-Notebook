use super::*;
use crate::backup::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupArtifactKind, BackupArtifactProtectionInput,
    BackupBlobRef, BackupBranchManifestInput, BackupBranchManifestPackRef, BackupLocator,
    BackupPackPlanInput, BackupProtectionMechanism, BackupSeqRange, parse_backup_key_ref,
    plan_backup_artifact_protection, plan_backup_pack, validate_backup_branch_manifest,
};

fn artifact_key(seed: u8) -> BackupArtifactKey {
    BackupArtifactKey::from_bytes(&[seed; 32]).unwrap()
}

fn protection(
    kind: BackupArtifactKind,
    mechanism: BackupProtectionMechanism,
) -> BackupArtifactProtection {
    plan_backup_artifact_protection(BackupArtifactProtectionInput {
        artifact_kind: kind,
        key_ref: parse_backup_key_ref("keyring:deve/default-backup-key").unwrap(),
        encrypted: true,
        authenticated: true,
        mechanism,
    })
    .unwrap()
}

fn encrypt_input<'a>(
    backup_key: &'a BackupArtifactKey,
    protection: &'a BackupArtifactProtection,
    plaintext: &'a [u8],
) -> BackupPackArtifactInput<'a> {
    BackupPackArtifactInput {
        repo_id: uuid::Uuid::from_u128(77),
        writer_identity: "writer-1",
        branch_path: "deve/branches/writer-1",
        pack_sequence: 3,
        protection,
        key: backup_key,
        plaintext,
    }
}

fn manifest_for(artifact: &BackupEncryptedPackArtifact) -> BackupPackManifest {
    plan_backup_pack(BackupPackPlanInput {
        repo_id: artifact.repo_id,
        writer_identity: artifact.writer_identity.clone(),
        branch_path: artifact.branch_path.clone(),
        pack_sequence: artifact.pack_sequence,
        ledger_seq_range: Some(BackupSeqRange { start: 1, end: 2 }),
        ledger_event_count: 2,
        snapshot_count: 1,
        payload_digest: artifact.payload_digest().unwrap(),
        blob_refs: vec![BackupBlobRef {
            path: "blobs/aa.bin".into(),
            size_bytes: 12,
            digest: artifact.payload_digest().unwrap(),
        }],
    })
    .unwrap()
}

fn branch_manifest_for(
    artifact: &BackupEncryptedPackArtifact,
) -> crate::backup::BackupBranchManifest {
    let locator = BackupLocator::parse("s3://bucket-name/deve/").unwrap();
    let branch = locator.branch_locator(&artifact.writer_identity).unwrap();
    validate_backup_branch_manifest(BackupBranchManifestInput {
        branch,
        expected_repo_id: artifact.repo_id,
        manifest_repo_id: artifact.repo_id,
        manifest_writer_identity: artifact.writer_identity.clone(),
        manifest_branch_path: artifact.branch_path.clone(),
        format_version: BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
        packs: vec![BackupBranchManifestPackRef::from_pack_manifest(
            &manifest_for(artifact),
        )],
    })
    .unwrap()
}

#[test]
fn backup_pack_artifact_roundtrips_encrypted_bytes() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let plaintext = br#"{"ledger":["event"],"snapshots":["snapshot"]}"#;

    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, plaintext)).unwrap();
    assert_ne!(artifact.ciphertext, plaintext);
    let artifact_bytes = artifact.to_bytes().unwrap();
    let manifest = manifest_for(&artifact);

    let opened = decrypt_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &manifest,
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .unwrap();

    assert_eq!(opened, plaintext);
}

#[test]
fn backup_pack_artifact_rejects_tamper_before_decrypt() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let plaintext = b"ledger facts";
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, plaintext)).unwrap();
    let manifest = manifest_for(&artifact);
    let mut tampered = artifact;
    tampered.ciphertext[0] ^= 0x01;
    let tampered_bytes = tampered.to_bytes().unwrap();

    let err = decrypt_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &manifest,
        key: &key,
        artifact_bytes: &tampered_bytes,
    })
    .expect_err("digest mismatch must stop before decrypt");

    assert_eq!(err, BackupPackArtifactError::ArtifactDigestMismatch);
}

#[test]
fn backup_pack_artifact_upload_verify_checks_digest_before_provider_io() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let manifest = manifest_for(&artifact);

    let digest = verify_backup_pack_artifact_for_upload(BackupPackArtifactUploadVerifyInput {
        manifest: &manifest,
        artifact_bytes: &artifact_bytes,
    })
    .unwrap();
    assert!(digest.same_sha256(&manifest.payload_digest));

    let mut tampered = artifact;
    tampered.ciphertext[0] ^= 0x01;
    let tampered_bytes = tampered.to_bytes().unwrap();
    let err = verify_backup_pack_artifact_for_upload(BackupPackArtifactUploadVerifyInput {
        manifest: &manifest,
        artifact_bytes: &tampered_bytes,
    })
    .expect_err("tampered upload payload must be rejected before provider PUT");
    assert_eq!(err, BackupPackArtifactError::ArtifactDigestMismatch);
}

#[test]
fn backup_pack_artifact_download_verify_checks_digest_before_decrypt() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let manifest = manifest_for(&artifact);

    let result =
        verify_downloaded_pack_artifact_digest_and_routing(BackupPackArtifactDownloadVerifyInput {
            manifest: &manifest,
            artifact_bytes: &artifact_bytes,
        })
        .unwrap();

    assert_eq!(result.pack_sequence(), artifact.pack_sequence);
    assert_eq!(result.object_path(), manifest.pack_object_path());
    assert!(
        result
            .computed_digest()
            .same_sha256(&manifest.payload_digest)
    );
}

#[test]
fn backup_pack_artifact_download_verify_rejects_tamper_before_decrypt() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let manifest = manifest_for(&artifact);
    let mut tampered = artifact;
    tampered.ciphertext[0] ^= 0x01;
    let tampered_bytes = tampered.to_bytes().unwrap();

    let err =
        verify_downloaded_pack_artifact_digest_and_routing(BackupPackArtifactDownloadVerifyInput {
            manifest: &manifest,
            artifact_bytes: &tampered_bytes,
        })
        .expect_err("tampered downloaded payload must be rejected before decrypt");

    assert_eq!(err, BackupPackArtifactError::ArtifactDigestMismatch);
}

#[test]
fn backup_pack_artifact_download_verify_rejects_routing_metadata_tamper() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let mut manifest = manifest_for(&artifact);
    let mut metadata_tampered = artifact;
    metadata_tampered.writer_identity = "writer-2".into();
    let tampered_bytes = metadata_tampered.to_bytes().unwrap();
    manifest.payload_digest = metadata_tampered.payload_digest().unwrap();

    let err =
        verify_downloaded_pack_artifact_digest_and_routing(BackupPackArtifactDownloadVerifyInput {
            manifest: &manifest,
            artifact_bytes: &tampered_bytes,
        })
        .expect_err("routing metadata drift must fail before decrypt");

    assert_eq!(err, BackupPackArtifactError::WriterIdentityMismatch);
}

#[test]
fn backup_pack_artifact_ref_download_verify_uses_branch_manifest_ref() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let branch_manifest = branch_manifest_for(&artifact);
    let pack_ref = &branch_manifest.packs[0];

    let result =
        verify_downloaded_pack_artifact_ref_and_routing(BackupPackArtifactRefDownloadVerifyInput {
            branch_manifest: &branch_manifest,
            pack_ref,
            artifact_bytes: &artifact_bytes,
        })
        .unwrap();

    assert_eq!(result.pack_sequence(), artifact.pack_sequence);
    assert_eq!(result.object_path(), pack_ref.object_path.as_str());
    assert!(
        result
            .computed_digest()
            .same_sha256(&pack_ref.payload_digest)
    );

    let mut tampered = artifact_bytes;
    tampered.push(b'\n');
    let err =
        verify_downloaded_pack_artifact_ref_and_routing(BackupPackArtifactRefDownloadVerifyInput {
            branch_manifest: &branch_manifest,
            pack_ref,
            artifact_bytes: &tampered,
        })
        .expect_err("tampered artifact must fail before decrypt");
    assert_eq!(err, BackupPackArtifactError::ArtifactDigestMismatch);
}

#[test]
fn backup_pack_artifact_ref_open_verifies_before_decrypt() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let plaintext = b"ledger facts from branch ref";
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, plaintext)).unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let branch_manifest = branch_manifest_for(&artifact);
    let pack_ref = &branch_manifest.packs[0];

    let opened = open_backup_pack_artifact_ref(BackupPackArtifactRefOpenInput {
        branch_manifest: &branch_manifest,
        pack_ref,
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .unwrap();

    assert_eq!(opened.pack_sequence(), artifact.pack_sequence);
    assert_eq!(opened.object_path(), pack_ref.object_path.as_str());
    assert!(
        opened
            .computed_digest()
            .same_sha256(&pack_ref.payload_digest)
    );
    assert_eq!(opened.plaintext(), plaintext);

    let mut tampered = artifact_bytes;
    tampered.push(b'\n');
    let err = open_backup_pack_artifact_ref(BackupPackArtifactRefOpenInput {
        branch_manifest: &branch_manifest,
        pack_ref,
        key: &key,
        artifact_bytes: &tampered,
    })
    .expect_err("digest mismatch must stop before decrypt");
    assert_eq!(err, BackupPackArtifactError::ArtifactDigestMismatch);
}

#[test]
fn backup_pack_artifact_ref_open_rejects_wrong_key_after_verified_digest() {
    let key = artifact_key(7);
    let wrong_key = artifact_key(8);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let branch_manifest = branch_manifest_for(&artifact);
    let pack_ref = &branch_manifest.packs[0];

    let err = open_backup_pack_artifact_ref(BackupPackArtifactRefOpenInput {
        branch_manifest: &branch_manifest,
        pack_ref,
        key: &wrong_key,
        artifact_bytes: &artifact_bytes,
    })
    .expect_err("wrong key must fail after digest/routing verification");

    assert_eq!(err, BackupPackArtifactError::DecryptFailed);
}

#[test]
fn backup_pack_artifact_rejects_wrong_key_after_verified_digest() {
    let key = artifact_key(7);
    let wrong_key = artifact_key(8);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let manifest = manifest_for(&artifact);

    let err = decrypt_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &manifest,
        key: &wrong_key,
        artifact_bytes: &artifact_bytes,
    })
    .expect_err("wrong key must fail closed");

    assert_eq!(err, BackupPackArtifactError::DecryptFailed);
}

#[test]
fn backup_pack_artifact_rejects_secret_and_metadata_drift() {
    assert!(matches!(
        BackupArtifactKey::from_bytes(&[1; 31]),
        Err(BackupPackArtifactError::InvalidKeyLength(31))
    ));

    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    assert!(matches!(
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"")),
        Err(BackupPackArtifactError::EmptyPlaintext)
    ));

    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let bytes = artifact.to_bytes().unwrap();
    let mut manifest = manifest_for(&artifact);
    manifest.writer_identity = "writer-2".into();

    let err = decrypt_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &manifest,
        key: &key,
        artifact_bytes: &bytes,
    })
    .expect_err("metadata drift must fail closed");

    assert_eq!(err, BackupPackArtifactError::WriterIdentityMismatch);
}

#[test]
fn backup_pack_artifact_rejects_unknown_plaintext_fields_before_decrypt() {
    let key = artifact_key(7);
    let protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let artifact =
        encrypt_backup_pack_artifact(encrypt_input(&key, &protection, b"ledger facts")).unwrap();
    let mut manifest = manifest_for(&artifact);
    let mut value = serde_json::to_value(&artifact).unwrap();
    value["key_material"] = serde_json::Value::String("must-not-be-accepted".into());
    let bytes = serde_json::to_vec(&value).unwrap();
    manifest.payload_digest = sha256_digest(&bytes);

    let err = decrypt_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &manifest,
        key: &key,
        artifact_bytes: &bytes,
    })
    .expect_err("unknown plaintext field must fail before decrypt");

    assert_eq!(err, BackupPackArtifactError::DeserializeFailed);
}

#[test]
fn backup_pack_artifact_requires_pack_aead_protection() {
    let key = artifact_key(7);
    let plaintext = b"ledger facts";
    let branch_manifest_protection = protection(
        BackupArtifactKind::BranchManifest,
        BackupProtectionMechanism::AeadTag,
    );
    let signature_protection = protection(
        BackupArtifactKind::Pack,
        BackupProtectionMechanism::Signature,
    );

    assert_eq!(
        encrypt_backup_pack_artifact(encrypt_input(&key, &branch_manifest_protection, plaintext))
            .expect_err("non-pack protection must fail"),
        BackupPackArtifactError::ProtectionKindMismatch
    );
    assert_eq!(
        encrypt_backup_pack_artifact(encrypt_input(&key, &signature_protection, plaintext))
            .expect_err("signature-only protection cannot encrypt pack bytes"),
        BackupPackArtifactError::UnsupportedProtectionMechanism
    );
}
