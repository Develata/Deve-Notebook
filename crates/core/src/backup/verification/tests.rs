use super::*;
use crate::models::RepoId;

fn digest(fill: char) -> BackupDigest {
    BackupDigest::sha256(fill.to_string().repeat(64))
}

fn uppercase_digest(fill: char) -> BackupDigest {
    BackupDigest::sha256(fill.to_ascii_uppercase().to_string().repeat(64))
}

fn repo_id() -> RepoId {
    "11111111-1111-1111-1111-111111111111"
        .parse()
        .expect("valid repo id")
}

fn pack() -> BackupPackVerificationEvidence {
    BackupPackVerificationEvidence {
        pack_sequence: 1,
        expected_digest: digest('b'),
        computed_digest: digest('b'),
        authenticated: true,
        decrypted: true,
    }
}

fn input() -> BackupVerificationInput {
    BackupVerificationInput {
        expected_repo_id: repo_id(),
        manifest_repo_id: repo_id(),
        expected_manifest_digest: digest('a'),
        computed_manifest_digest: digest('a'),
        manifest_authenticated: true,
        packs: vec![pack()],
        decrypt_required: true,
    }
}

#[test]
fn verifies_authenticated_decrypted_pack_artifacts() {
    let result = verify_backup_artifacts(input()).expect("verified artifacts");

    assert_eq!(result.repo_id(), repo_id());
    assert_eq!(result.manifest_digest(), &digest('a'));
    assert_eq!(result.pack_count(), 1);
    assert_eq!(result.pack_digests(), &[digest('b')]);
    assert!(result.decrypted());
}

#[test]
fn accepts_canonical_sha256_case_differences() {
    let mut input = input();
    input.expected_manifest_digest = uppercase_digest('a');
    input.packs[0].expected_digest = uppercase_digest('b');

    let result = verify_backup_artifacts(input).expect("case-insensitive digest match");

    assert_eq!(result.pack_count(), 1);
    assert!(result.decrypted());
}

#[test]
fn rejects_repo_id_mismatch() {
    let mut input = input();
    input.manifest_repo_id = "22222222-2222-2222-2222-222222222222"
        .parse()
        .expect("valid repo id");

    let err = verify_backup_artifacts(input).expect_err("repo mismatch");

    assert_eq!(err, BackupVerificationError::RepoIdMismatch);
}

#[test]
fn rejects_manifest_hash_mismatch() {
    let mut input = input();
    input.computed_manifest_digest = digest('c');

    let err = verify_backup_artifacts(input).expect_err("manifest hash mismatch");

    assert_eq!(err, BackupVerificationError::ManifestHashMismatch);
}

#[test]
fn rejects_pack_hash_mismatch() {
    let mut input = input();
    input.packs[0].computed_digest = digest('c');
    input.packs[0].decrypted = false;

    let err = verify_backup_artifacts(input).expect_err("pack hash mismatch");

    assert_eq!(err, BackupVerificationError::PackHashMismatch);
}

#[test]
fn rejects_duplicate_pack_sequence() {
    let mut input = input();
    input.packs.push(pack());

    let err = verify_backup_artifacts(input).expect_err("duplicate pack sequence");

    assert_eq!(err, BackupVerificationError::DuplicatePackSequence);
}

#[test]
fn rejects_decrypt_before_hash_and_authentication_verify() {
    let mut input = input();
    input.packs[0].computed_digest = digest('c');

    let err = verify_backup_artifacts(input).expect_err("decrypt before verify");

    assert_eq!(err, BackupVerificationError::DecryptBeforeVerifyForbidden);
}

#[test]
fn rejects_pack_authentication_failure_before_decrypt() {
    let mut input = input();
    input.packs[0].authenticated = false;
    input.packs[0].decrypted = false;

    let err = verify_backup_artifacts(input).expect_err("pack auth failure");

    assert_eq!(err, BackupVerificationError::PackAuthenticationFailed);
}

#[test]
fn requires_decrypt_when_restore_path_needs_plaintext() {
    let mut input = input();
    input.packs[0].decrypted = false;

    let err = verify_backup_artifacts(input).expect_err("decrypt required");

    assert_eq!(err, BackupVerificationError::DecryptFailure);
}
