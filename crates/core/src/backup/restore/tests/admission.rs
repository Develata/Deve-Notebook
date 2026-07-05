use super::super::{
    BackupRestoreError, RestoreAdmissionMode, RestoreAdmissionState,
    RestoreCandidateFromVerifiedPacksInput, admit_restore_candidate,
    admit_verified_restore_candidate,
};
use super::{
    digest, input, manifest_verification, manifest_verification_with_sequences, repo_id,
    verified_restore_evidence,
};

#[test]
fn admits_remote_readonly_candidate_after_verify_download_decrypt() {
    let candidate = admit_restore_candidate(input()).unwrap();

    assert_eq!(candidate.writer_identity, "writer-1");
    assert_eq!(candidate.branch_path, "deve/branches/writer-1");
    assert_eq!(candidate.pack_count, 2);
    assert_eq!(candidate.state, RestoreAdmissionState::RemoteReadonly);
}

#[test]
fn backup_restore_candidate_admission_consumes_verified_plaintext_pack_evidence() {
    let evidence = verified_restore_evidence();

    let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &evidence.manifest_verification,
        plaintext_packs: &evidence.plaintext_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect("restore candidate from typed evidence");

    assert_eq!(candidate.repo_id, repo_id());
    assert_eq!(
        candidate.writer_identity,
        evidence.plaintext_packs.writer_identity()
    );
    assert_eq!(
        candidate.branch_path,
        evidence.plaintext_packs.branch_path()
    );
    assert_eq!(
        &candidate.manifest_digest,
        evidence.manifest_verification.manifest_digest()
    );
    assert_eq!(candidate.pack_count, evidence.plaintext_packs.pack_count());
    assert_eq!(
        candidate.pack_digests.as_slice(),
        evidence.plaintext_packs.pack_digests()
    );
    assert_eq!(candidate.state, RestoreAdmissionState::RemoteReadonly);
}

#[test]
fn backup_restore_candidate_admission_preserves_repo_and_write_gates() {
    let evidence = verified_restore_evidence();

    let err = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: uuid::Uuid::from_u128(7),
        manifest_verification: &evidence.manifest_verification,
        plaintext_packs: &evidence.plaintext_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect_err("repo mismatch must fail closed");
    assert_eq!(err, BackupRestoreError::RepoIdMismatch);

    let err = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &evidence.manifest_verification,
        plaintext_packs: &evidence.plaintext_packs,
        admission_mode: RestoreAdmissionMode::ExplicitImport,
        write_gate_confirmed: false,
    })
    .expect_err("explicit import must require write gate");
    assert_eq!(err, BackupRestoreError::WriteGateRequired);
}

#[test]
fn backup_restore_candidate_admission_rejects_manifest_and_plaintext_pack_mismatch() {
    let evidence = verified_restore_evidence();
    let mismatched_manifest = manifest_verification(vec![digest('d'), digest('e')]);

    let err = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &mismatched_manifest,
        plaintext_packs: &evidence.plaintext_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect_err("manifest/plaintext pack mismatch must fail closed");
    assert_eq!(err, BackupRestoreError::TypedEvidenceMismatch);

    let sequence_mismatch = manifest_verification_with_sequences(vec![
        (2, evidence.plaintext_packs.pack_digests()[0].clone()),
        (1, evidence.plaintext_packs.pack_digests()[1].clone()),
    ]);
    let err = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &sequence_mismatch,
        plaintext_packs: &evidence.plaintext_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect_err("manifest/plaintext pack sequence mismatch must fail closed");
    assert_eq!(err, BackupRestoreError::TypedEvidenceMismatch);
}

#[test]
fn backup_restore_candidate_admission_writes_no_local_authority() {
    let evidence = verified_restore_evidence();

    let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &evidence.manifest_verification,
        plaintext_packs: &evidence.plaintext_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect("remote readonly admission is metadata only");

    assert_eq!(candidate.state, RestoreAdmissionState::RemoteReadonly);
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

    assert!(matches!(
        admit_restore_candidate(candidate_input.clone()),
        Err(BackupRestoreError::WriteGateRequired)
    ));

    candidate_input.write_gate_confirmed = true;
    let candidate = admit_restore_candidate(candidate_input).unwrap();
    assert_eq!(candidate.state, RestoreAdmissionState::ExplicitMerge);
}
