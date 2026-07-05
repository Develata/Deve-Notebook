use super::{
    BackupDigest, BackupRestoreError, RestoreAdmissionMode, RestoreAdmissionState,
    RestoreCandidateFromVerifiedPacksInput, RestoreCandidateInput, RestoreEvidence,
    admit_restore_candidate, admit_verified_restore_candidate,
};
use crate::backup::BackupLocator;
use crate::backup::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupArtifactKey, BackupArtifactKind,
    BackupArtifactProtection, BackupArtifactProtectionInput, BackupBlobRef, BackupBranchManifest,
    BackupBranchManifestPackRef, BackupEncryptedPackArtifact,
    BackupPackArtifactDownloadVerifyInput, BackupPackArtifactInput, BackupPackArtifactOpenInput,
    BackupPackArtifactOpenResult, BackupPackManifest, BackupPackPlanInput,
    BackupPackVerificationEvidence, BackupProtectionMechanism, BackupSeqRange,
    BackupVerificationInput, BackupVerificationResult, encrypt_backup_pack_artifact,
    open_backup_pack_artifact, parse_backup_key_ref, plan_backup_artifact_protection,
    plan_backup_pack, verify_backup_artifacts, verify_decrypted_backup_packs,
    verify_downloaded_backup_packs, verify_downloaded_pack_artifact_digest_and_routing,
};
use crate::models::RepoId;

struct PackFixture {
    download_result: crate::backup::BackupPackArtifactDownloadVerifyResult,
    open_result: BackupPackArtifactOpenResult,
}

struct RestoreEvidenceFixture {
    manifest_verification: BackupVerificationResult,
    decrypted_packs: crate::backup::BackupDecryptedPacksResult,
}

fn repo_id() -> RepoId {
    uuid::Uuid::from_u128(42)
}

fn digest(seed: char) -> BackupDigest {
    BackupDigest::sha256(seed.to_string().repeat(64))
}

fn uppercase_digest(seed: char) -> BackupDigest {
    BackupDigest::sha256(seed.to_ascii_uppercase().to_string().repeat(64))
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

fn pack_manifest_for(artifact: &BackupEncryptedPackArtifact) -> BackupPackManifest {
    let payload_digest = artifact.payload_digest().unwrap();
    plan_backup_pack(BackupPackPlanInput {
        repo_id: artifact.repo_id,
        writer_identity: artifact.writer_identity.clone(),
        branch_path: artifact.branch_path.clone(),
        pack_sequence: artifact.pack_sequence,
        ledger_seq_range: Some(BackupSeqRange {
            start: artifact.pack_sequence,
            end: artifact.pack_sequence,
        }),
        ledger_event_count: 1,
        snapshot_count: 1,
        payload_digest: payload_digest.clone(),
        blob_refs: vec![BackupBlobRef {
            path: format!("blobs/{:06}.bin", artifact.pack_sequence),
            size_bytes: 12,
            digest: payload_digest,
        }],
    })
    .unwrap()
}

fn pack_fixture(pack_sequence: u64, writer_identity: &str, plaintext: &[u8]) -> PackFixture {
    let key = artifact_key();
    let protection = protection();
    let branch_path = format!("deve/branches/{writer_identity}");
    let artifact = encrypt_backup_pack_artifact(pack_artifact_input(
        &key,
        &protection,
        pack_sequence,
        writer_identity,
        &branch_path,
        plaintext,
    ))
    .unwrap();
    let artifact_bytes = artifact.to_bytes().unwrap();
    let manifest = pack_manifest_for(&artifact);
    let download_result =
        verify_downloaded_pack_artifact_digest_and_routing(BackupPackArtifactDownloadVerifyInput {
            manifest: &manifest,
            artifact_bytes: &artifact_bytes,
        })
        .unwrap();
    let open_result = open_backup_pack_artifact(BackupPackArtifactOpenInput {
        manifest: &manifest,
        key: &key,
        artifact_bytes: &artifact_bytes,
    })
    .unwrap();

    PackFixture {
        download_result,
        open_result,
    }
}

fn pack_fixtures() -> Vec<PackFixture> {
    vec![
        pack_fixture(1, "writer-1", b"ledger facts one"),
        pack_fixture(2, "writer-1", b"ledger facts two"),
    ]
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
            .map(|fixture| BackupBranchManifestPackRef {
                pack_sequence: fixture.download_result.pack_sequence(),
                object_path: fixture.download_result.object_path().to_owned(),
                payload_digest: fixture.download_result.computed_digest().clone(),
            })
            .collect(),
    }
}

fn manifest_verification(pack_digests: Vec<BackupDigest>) -> BackupVerificationResult {
    let packs = pack_digests
        .into_iter()
        .enumerate()
        .map(|(index, digest)| BackupPackVerificationEvidence {
            pack_sequence: u64::try_from(index + 1).unwrap(),
            expected_digest: digest.clone(),
            computed_digest: digest,
            authenticated: true,
            decrypted: true,
        })
        .collect();
    verify_backup_artifacts(BackupVerificationInput {
        expected_repo_id: repo_id(),
        manifest_repo_id: repo_id(),
        expected_manifest_digest: digest('a'),
        computed_manifest_digest: digest('a'),
        manifest_authenticated: true,
        packs,
        decrypt_required: true,
    })
    .unwrap()
}

fn verified_restore_evidence() -> RestoreEvidenceFixture {
    let fixtures = pack_fixtures();
    let manifest = branch_manifest(&fixtures);
    let downloaded = verify_downloaded_backup_packs(crate::backup::BackupDownloadedPacksInput {
        branch_manifest: &manifest,
        verified_packs: fixtures
            .iter()
            .map(|fixture| fixture.download_result.clone())
            .collect(),
    })
    .unwrap();
    let manifest_verification = manifest_verification(downloaded.pack_digests().to_vec());
    let decrypted_packs = verify_decrypted_backup_packs(crate::backup::BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs: fixtures
            .into_iter()
            .map(|fixture| fixture.open_result)
            .collect(),
    })
    .unwrap();

    RestoreEvidenceFixture {
        manifest_verification,
        decrypted_packs,
    }
}

fn input() -> RestoreCandidateInput {
    let branch = BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap();

    RestoreCandidateInput {
        repo_id: repo_id(),
        expected_repo_id: repo_id(),
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
fn backup_restore_candidate_admission_consumes_verified_and_decrypted_evidence() {
    let evidence = verified_restore_evidence();

    let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &evidence.manifest_verification,
        decrypted_packs: &evidence.decrypted_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect("restore candidate from typed evidence");

    assert_eq!(candidate.repo_id, repo_id());
    assert_eq!(
        candidate.writer_identity,
        evidence.decrypted_packs.writer_identity()
    );
    assert_eq!(
        candidate.branch_path,
        evidence.decrypted_packs.branch_path()
    );
    assert_eq!(
        &candidate.manifest_digest,
        evidence.manifest_verification.manifest_digest()
    );
    assert_eq!(candidate.pack_count, evidence.decrypted_packs.pack_count());
    assert_eq!(
        candidate.pack_digests.as_slice(),
        evidence.decrypted_packs.pack_digests()
    );
    assert_eq!(candidate.state, RestoreAdmissionState::RemoteReadonly);
}

#[test]
fn backup_restore_candidate_admission_preserves_repo_and_write_gates() {
    let evidence = verified_restore_evidence();

    let err = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: uuid::Uuid::from_u128(7),
        manifest_verification: &evidence.manifest_verification,
        decrypted_packs: &evidence.decrypted_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect_err("repo mismatch must fail closed");
    assert_eq!(err, BackupRestoreError::RepoIdMismatch);

    let err = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &evidence.manifest_verification,
        decrypted_packs: &evidence.decrypted_packs,
        admission_mode: RestoreAdmissionMode::ExplicitImport,
        write_gate_confirmed: false,
    })
    .expect_err("explicit import must require write gate");
    assert_eq!(err, BackupRestoreError::WriteGateRequired);
}

#[test]
fn backup_restore_candidate_admission_rejects_manifest_and_decrypted_pack_mismatch() {
    let evidence = verified_restore_evidence();
    let mismatched_manifest = manifest_verification(vec![digest('d'), digest('e')]);

    let err = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &mismatched_manifest,
        decrypted_packs: &evidence.decrypted_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect_err("manifest/decrypted pack mismatch must fail closed");
    assert_eq!(err, BackupRestoreError::TypedEvidenceMismatch);
}

#[test]
fn backup_restore_candidate_admission_writes_no_local_authority() {
    let evidence = verified_restore_evidence();

    let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: repo_id(),
        manifest_verification: &evidence.manifest_verification,
        decrypted_packs: &evidence.decrypted_packs,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })
    .expect("remote readonly admission is metadata only");

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
