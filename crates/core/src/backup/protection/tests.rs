use super::*;
use crate::backup::{BackupSecretRefKind, parse_backup_credential_ref, parse_backup_key_ref};

fn input() -> BackupArtifactProtectionInput {
    BackupArtifactProtectionInput {
        artifact_kind: BackupArtifactKind::Pack,
        key_ref: parse_backup_key_ref("keyring:deve/default-backup-key").unwrap(),
        encrypted: true,
        authenticated: true,
        mechanism: BackupProtectionMechanism::AeadTag,
    }
}

#[test]
fn admits_encrypted_authenticated_pack_with_key_ref() {
    let protection = plan_backup_artifact_protection(input()).expect("protection");

    assert_eq!(protection.artifact_kind(), BackupArtifactKind::Pack);
    assert_eq!(protection.key_ref().kind, BackupSecretRefKind::Key);
    assert_eq!(protection.mechanism(), BackupProtectionMechanism::AeadTag);
}

#[test]
fn admits_signature_protected_manifest_metadata() {
    let mut input = input();
    input.artifact_kind = BackupArtifactKind::RepoManifest;
    input.mechanism = BackupProtectionMechanism::Signature;

    let protection = plan_backup_artifact_protection(input).expect("protection");

    assert_eq!(protection.artifact_kind(), BackupArtifactKind::RepoManifest);
    assert_eq!(protection.mechanism(), BackupProtectionMechanism::Signature);
}

#[test]
fn rejects_credential_ref_as_artifact_key() {
    let mut input = input();
    input.key_ref = parse_backup_credential_ref("env:DEVE_BACKUP_TOKEN").unwrap();

    let err = plan_backup_artifact_protection(input).expect_err("credential ref");

    assert_eq!(err, BackupArtifactProtectionError::KeyRefKindMismatch);
}

#[test]
fn rejects_unencrypted_artifact() {
    let mut input = input();
    input.encrypted = false;

    let err = plan_backup_artifact_protection(input).expect_err("unencrypted");

    assert_eq!(err, BackupArtifactProtectionError::ArtifactMustBeEncrypted);
}

#[test]
fn rejects_unauthenticated_artifact() {
    let mut input = input();
    input.authenticated = false;

    let err = plan_backup_artifact_protection(input).expect_err("unauthenticated");

    assert_eq!(
        err,
        BackupArtifactProtectionError::ArtifactMustBeAuthenticated
    );
}
