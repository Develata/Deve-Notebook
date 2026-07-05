use super::super::{BackupDigest, BackupRestoreError, admit_restore_candidate};
use super::{input, uppercase_digest};

#[test]
fn rejects_repo_id_mismatch() {
    let mut input = input();
    input.expected_repo_id = uuid::Uuid::from_u128(7);

    assert!(matches!(
        admit_restore_candidate(input),
        Err(BackupRestoreError::RepoIdMismatch)
    ));
}

#[test]
fn rejects_candidate_before_decrypt() {
    let mut input = input();
    input.evidence.packs_decrypted = false;

    assert!(matches!(
        admit_restore_candidate(input),
        Err(BackupRestoreError::IncompleteRestoreEvidence)
    ));
}

#[test]
fn rejects_bad_pack_digest_or_count() {
    let mut candidate_input = input();
    candidate_input.pack_count = 3;
    assert!(matches!(
        admit_restore_candidate(candidate_input),
        Err(BackupRestoreError::PackDigestCountMismatch)
    ));

    let mut candidate_input = input();
    candidate_input.pack_digests[0] = BackupDigest::sha256("not-hex");
    assert!(matches!(
        admit_restore_candidate(candidate_input),
        Err(BackupRestoreError::InvalidDigest)
    ));

    let mut candidate_input = input();
    candidate_input.pack_count = 0;
    candidate_input.pack_digests.clear();
    assert!(matches!(
        admit_restore_candidate(candidate_input),
        Err(BackupRestoreError::EmptyRestoreCandidate)
    ));

    let mut candidate_input = input();
    candidate_input.pack_digests[1] = candidate_input.pack_digests[0].clone();
    assert!(matches!(
        admit_restore_candidate(candidate_input),
        Err(BackupRestoreError::DuplicatePackDigest)
    ));

    let mut candidate_input = input();
    candidate_input.pack_digests[1] = uppercase_digest('b');
    assert!(matches!(
        admit_restore_candidate(candidate_input),
        Err(BackupRestoreError::DuplicatePackDigest)
    ));
}

#[test]
fn rejects_unsafe_writer_or_branch_path() {
    let mut candidate_input = input();
    candidate_input.writer_identity = "../writer".into();
    assert!(admit_restore_candidate(candidate_input).is_err());

    let mut candidate_input = input();
    candidate_input.branch_path = "deve//branches/writer-1".into();
    assert!(admit_restore_candidate(candidate_input).is_err());
}
