use super::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupBranchManifestArtifactError,
    BackupBranchManifestArtifactInput, BackupBranchManifestArtifactOpenInput,
    BackupBranchManifestError, BackupBranchManifestInput, BackupBranchManifestPackRef,
    encrypt_backup_branch_manifest_artifact, open_backup_branch_manifest_artifact,
    validate_backup_branch_manifest,
};
use crate::backup::{
    BackupArtifactKey, BackupArtifactKind, BackupArtifactProtection, BackupArtifactProtectionInput,
    BackupDigest, BackupLocator, BackupProtectionMechanism, parse_backup_key_ref,
    plan_backup_artifact_protection,
};

fn repo_id() -> uuid::Uuid {
    uuid::Uuid::from_u128(23)
}

fn digest(seed: char) -> BackupDigest {
    BackupDigest::sha256(seed.to_string().repeat(64))
}

fn branch() -> crate::backup::BranchBackupLocator {
    BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap()
}

fn pack(sequence: u64) -> BackupBranchManifestPackRef {
    BackupBranchManifestPackRef {
        pack_sequence: sequence,
        object_path: format!("deve/branches/writer-1/packs/{sequence:06}.pack.enc"),
        payload_digest: digest('a'),
    }
}

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

fn manifest_input() -> BackupBranchManifestInput {
    BackupBranchManifestInput {
        branch: branch(),
        expected_repo_id: repo_id(),
        manifest_repo_id: repo_id(),
        manifest_writer_identity: "writer-1".into(),
        manifest_branch_path: "deve/branches/writer-1/".into(),
        format_version: BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
        packs: vec![pack(1), pack(2)],
    }
}

fn artifact_input<'a>(
    key: &'a BackupArtifactKey,
    protection: &'a BackupArtifactProtection,
) -> BackupBranchManifestArtifactInput<'a> {
    BackupBranchManifestArtifactInput {
        branch: branch(),
        repo_id: repo_id(),
        writer_identity: "writer-1",
        branch_path: "deve/branches/writer-1",
        packs: vec![pack(1), pack(2)],
        protection,
        key,
    }
}

#[test]
fn validates_branch_manifest_and_expected_pack_paths() {
    let manifest = validate_backup_branch_manifest(manifest_input()).expect("branch manifest");

    assert_eq!(manifest.repo_id, repo_id());
    assert_eq!(manifest.writer_identity, "writer-1");
    assert_eq!(manifest.branch_path, "deve/branches/writer-1");
    assert_eq!(
        manifest.branch_manifest_path,
        "deve/branches/writer-1/branch.manifest.enc"
    );
    assert_eq!(
        manifest.expected_pack_object_paths(),
        vec![
            "deve/branches/writer-1/packs/000001.pack.enc".to_string(),
            "deve/branches/writer-1/packs/000002.pack.enc".to_string()
        ]
    );
}

#[test]
fn rejects_repo_writer_branch_and_format_mismatch() {
    let mut input = manifest_input();
    input.format_version = 99;
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::UnsupportedFormatVersion(99))
    ));

    let mut input = manifest_input();
    input.manifest_repo_id = uuid::Uuid::from_u128(24);
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::RepoIdMismatch)
    ));

    let mut input = manifest_input();
    input.manifest_writer_identity = "writer-2".into();
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::WriterIdentityMismatch)
    ));

    let mut input = manifest_input();
    input.manifest_branch_path = "deve/branches/writer-2".into();
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::BranchPathMismatch)
    ));
}

#[test]
fn rejects_duplicate_pack_sequence_and_object_path() {
    let mut input = manifest_input();
    input.packs = vec![pack(1), pack(1)];
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::DuplicatePackSequence)
    ));

    let mut input = manifest_input();
    input.packs = vec![
        BackupBranchManifestPackRef {
            pack_sequence: 1,
            object_path: "deve/branches/writer-1/packs/000001.pack.enc".into(),
            payload_digest: digest('a'),
        },
        BackupBranchManifestPackRef {
            pack_sequence: 2,
            object_path: "deve/branches/writer-1/packs/000001.pack.enc".into(),
            payload_digest: digest('b'),
        },
    ];
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::DuplicatePackObjectPath)
    ));
}

#[test]
fn rejects_unsafe_or_mismatched_pack_refs() {
    let mut input = manifest_input();
    input.packs.clear();
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::EmptyPackList)
    ));

    let mut input = manifest_input();
    input.packs[0].pack_sequence = 0;
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::InvalidPackSequence)
    ));

    let mut input = manifest_input();
    input.packs[0].object_path = "deve/branches/writer-2/packs/000001.pack.enc".into();
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::PackPathOutsideBranchPrefix)
    ));

    let mut input = manifest_input();
    input.packs[0].object_path = "deve/branches/writer-1/packs/000099.pack.enc".into();
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::PackObjectPathMismatch)
    ));

    let mut input = manifest_input();
    input.packs[0].payload_digest = BackupDigest::sha256("not-hex");
    assert!(matches!(
        validate_backup_branch_manifest(input),
        Err(BackupBranchManifestError::InvalidDigest)
    ));
}

#[test]
fn backup_branch_manifest_artifact_roundtrips_after_digest_and_authentication() {
    let key = artifact_key(5);
    let protection = protection(
        BackupArtifactKind::BranchManifest,
        BackupProtectionMechanism::AeadTag,
    );
    let artifact = encrypt_backup_branch_manifest_artifact(artifact_input(&key, &protection))
        .expect("encrypted branch manifest");
    let artifact_bytes = artifact.to_bytes().expect("artifact bytes");
    let expected_manifest_digest = artifact.payload_digest().expect("artifact digest");

    let opened = open_backup_branch_manifest_artifact(BackupBranchManifestArtifactOpenInput {
        branch: branch(),
        expected_repo_id: repo_id(),
        expected_manifest_digest,
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .expect("opened branch manifest");

    assert_eq!(opened.branch_manifest().repo_id, repo_id());
    assert_eq!(opened.branch_manifest().writer_identity, "writer-1");
    assert_eq!(
        opened.branch_manifest().expected_pack_object_paths(),
        vec![
            "deve/branches/writer-1/packs/000001.pack.enc".to_string(),
            "deve/branches/writer-1/packs/000002.pack.enc".to_string()
        ]
    );
    assert!(
        opened
            .computed_digest()
            .same_sha256(&artifact.payload_digest().unwrap())
    );
}

#[test]
fn backup_branch_manifest_artifact_rejects_tamper_before_decrypt() {
    let key = artifact_key(5);
    let protection = protection(
        BackupArtifactKind::BranchManifest,
        BackupProtectionMechanism::AeadTag,
    );
    let artifact = encrypt_backup_branch_manifest_artifact(artifact_input(&key, &protection))
        .expect("encrypted branch manifest");
    let expected_manifest_digest = artifact.payload_digest().expect("artifact digest");
    let mut artifact_bytes = artifact.to_bytes().expect("artifact bytes");
    artifact_bytes.push(b'\n');

    let err = open_backup_branch_manifest_artifact(BackupBranchManifestArtifactOpenInput {
        branch: branch(),
        expected_repo_id: repo_id(),
        expected_manifest_digest,
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .expect_err("tamper must fail before decrypt");

    assert_eq!(
        err,
        BackupBranchManifestArtifactError::ArtifactDigestMismatch
    );
}

#[test]
fn backup_branch_manifest_artifact_rejects_wrong_key_after_verified_digest() {
    let key = artifact_key(5);
    let wrong_key = artifact_key(6);
    let protection = protection(
        BackupArtifactKind::BranchManifest,
        BackupProtectionMechanism::AeadTag,
    );
    let artifact = encrypt_backup_branch_manifest_artifact(artifact_input(&key, &protection))
        .expect("encrypted branch manifest");
    let artifact_bytes = artifact.to_bytes().expect("artifact bytes");

    let err = open_backup_branch_manifest_artifact(BackupBranchManifestArtifactOpenInput {
        branch: branch(),
        expected_repo_id: repo_id(),
        expected_manifest_digest: artifact.payload_digest().expect("artifact digest"),
        key: &wrong_key,
        artifact_bytes: &artifact_bytes,
    })
    .expect_err("wrong key must fail closed");

    assert_eq!(err, BackupBranchManifestArtifactError::DecryptFailed);
}

#[test]
fn backup_branch_manifest_artifact_rejects_routing_and_protection_mismatch() {
    let key = artifact_key(5);
    let branch_manifest_protection = protection(
        BackupArtifactKind::BranchManifest,
        BackupProtectionMechanism::AeadTag,
    );
    let artifact =
        encrypt_backup_branch_manifest_artifact(artifact_input(&key, &branch_manifest_protection))
            .expect("encrypted branch manifest");
    let artifact_bytes = artifact.to_bytes().expect("artifact bytes");
    let err = open_backup_branch_manifest_artifact(BackupBranchManifestArtifactOpenInput {
        branch: BackupLocator::parse("s3://bucket-name/deve/")
            .unwrap()
            .branch_locator("writer-2")
            .unwrap(),
        expected_repo_id: repo_id(),
        expected_manifest_digest: artifact.payload_digest().expect("artifact digest"),
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .expect_err("writer routing mismatch must fail closed");
    assert_eq!(
        err,
        BackupBranchManifestArtifactError::WriterIdentityMismatch
    );

    let pack_protection = protection(BackupArtifactKind::Pack, BackupProtectionMechanism::AeadTag);
    let err = encrypt_backup_branch_manifest_artifact(artifact_input(&key, &pack_protection))
        .expect_err("pack protection cannot seal branch manifest");
    assert_eq!(
        err,
        BackupBranchManifestArtifactError::ProtectionKindMismatch
    );

    let signature_protection = protection(
        BackupArtifactKind::BranchManifest,
        BackupProtectionMechanism::Signature,
    );
    let err = encrypt_backup_branch_manifest_artifact(artifact_input(&key, &signature_protection))
        .expect_err("signature-only protection cannot encrypt branch manifest");
    assert_eq!(
        err,
        BackupBranchManifestArtifactError::UnsupportedProtectionMechanism
    );
}
