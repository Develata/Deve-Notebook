use super::{
    BackupDigest, BackupRestoreError, RestoreAdmissionMode, RestoreAdmissionState,
    RestoreCandidateInput, RestoreEvidence, admit_restore_candidate,
};
use crate::backup::BackupLocator;

fn digest(seed: char) -> BackupDigest {
    BackupDigest::sha256(seed.to_string().repeat(64))
}

fn input() -> RestoreCandidateInput {
    let repo_id = uuid::Uuid::from_u128(42);
    let branch = BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap();

    RestoreCandidateInput {
        repo_id,
        expected_repo_id: repo_id,
        writer_identity: branch.writer_identity,
        branch_path: format!("{}/", branch.branch_path),
        manifest_digest: digest('a'),
        pack_count: 2,
        pack_digests: vec![digest('b'), digest('c')],
        evidence: RestoreEvidence::verified_downloaded_decrypted(),
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    }
}

#[test]
fn admits_remote_readonly_candidate_after_verify_download_decrypt() {
    let candidate = admit_restore_candidate(input()).unwrap();

    assert_eq!(candidate.writer_identity, "writer-1");
    assert_eq!(candidate.branch_path, "deve/branches/writer-1");
    assert_eq!(candidate.pack_count, 2);
    assert_eq!(candidate.state, RestoreAdmissionState::RemoteReadonly);
}

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
fn explicit_import_and_merge_require_write_gate() {
    let mut candidate_input = input();
    candidate_input.admission_mode = RestoreAdmissionMode::ExplicitImport;

    assert!(matches!(
        admit_restore_candidate(candidate_input.clone()),
        Err(BackupRestoreError::WriteGateRequired)
    ));

    candidate_input.write_gate_confirmed = true;
    let candidate = admit_restore_candidate(candidate_input).unwrap();
    assert_eq!(candidate.state, RestoreAdmissionState::ExplicitImport);

    let mut candidate_input = input();
    candidate_input.admission_mode = RestoreAdmissionMode::ExplicitMerge;
    candidate_input.write_gate_confirmed = true;
    let candidate = admit_restore_candidate(candidate_input).unwrap();
    assert_eq!(candidate.state, RestoreAdmissionState::ExplicitMerge);
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
