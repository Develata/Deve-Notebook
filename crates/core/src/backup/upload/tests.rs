use super::{
    BackupDigest, BackupPackError, BackupUploadError, BackupUploadEvidence, BackupUploadPlanInput,
    BackupUploadState, plan_backup_upload,
};
use crate::backup::{
    BackupArtifactKind, BackupArtifactProtection, BackupArtifactProtectionInput,
    BackupBindingAccess, BackupBlobRef, BackupBranchBinding, BackupBranchBindingInput,
    BackupLocator, BackupPackPlanInput, BackupProtectionMechanism, BackupSeqRange,
    parse_backup_key_ref, plan_backup_artifact_protection, plan_backup_branch_binding,
    plan_backup_pack,
};

fn digest(seed: char) -> BackupDigest {
    BackupDigest::sha256(seed.to_string().repeat(64))
}

fn binding(access: BackupBindingAccess) -> BackupBranchBinding {
    let branch = BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap();

    plan_backup_branch_binding(BackupBranchBindingInput {
        repo_id: uuid::Uuid::from_u128(42),
        branch_name: "main".into(),
        writer_identity: branch.writer_identity,
        local_writer_identity: "writer-1".into(),
        branch_path: branch.branch_path,
        requested_access: access,
    })
    .unwrap()
}

fn plan_input(evidence: BackupUploadEvidence) -> BackupUploadPlanInput {
    let binding = binding(BackupBindingAccess::Writable);
    let manifest = plan_backup_pack(BackupPackPlanInput {
        repo_id: binding.repo_id,
        writer_identity: binding.writer_identity.clone(),
        branch_path: binding.branch_path.clone(),
        pack_sequence: 7,
        ledger_seq_range: Some(BackupSeqRange { start: 10, end: 12 }),
        ledger_event_count: 3,
        snapshot_count: 1,
        payload_digest: digest('a'),
        blob_refs: vec![BackupBlobRef {
            path: "blobs/aa.bin".into(),
            size_bytes: 12,
            digest: digest('b'),
        }],
    })
    .unwrap();

    BackupUploadPlanInput {
        binding,
        manifest,
        protection: None,
        evidence,
    }
}

fn pack_protection() -> BackupArtifactProtection {
    plan_backup_artifact_protection(BackupArtifactProtectionInput {
        artifact_kind: BackupArtifactKind::Pack,
        key_ref: parse_backup_key_ref("keyring:deve/default-backup-key").unwrap(),
        encrypted: true,
        authenticated: true,
        mechanism: BackupProtectionMechanism::AeadTag,
    })
    .unwrap()
}

fn protected_plan_input(evidence: BackupUploadEvidence) -> BackupUploadPlanInput {
    let mut input = plan_input(evidence);
    input.protection = Some(pack_protection());
    input
}

#[test]
fn plans_pack_planned_state_for_writable_binding_and_manifest() {
    let plan = plan_backup_upload(plan_input(BackupUploadEvidence::none())).unwrap();

    assert_eq!(plan.branch_name, "main");
    assert_eq!(plan.branch_path, "deve/branches/writer-1");
    assert_eq!(
        plan.pack_object_path,
        "deve/branches/writer-1/packs/000007.pack.enc"
    );
    assert_eq!(plan.state, BackupUploadState::PackPlanned);
}

#[test]
fn progresses_through_encrypted_uploaded_verified_and_complete() {
    let mut evidence = BackupUploadEvidence::none();
    evidence.pack_encrypted = true;
    let plan = plan_backup_upload(protected_plan_input(evidence.clone())).unwrap();
    assert_eq!(plan.state, BackupUploadState::PackEncrypted);

    evidence.uploaded_payload_digest = Some(digest('a'));
    let plan = plan_backup_upload(protected_plan_input(evidence.clone())).unwrap();
    assert_eq!(plan.state, BackupUploadState::Uploaded);

    evidence.remote_manifest_payload_digest = Some(digest('a'));
    let plan = plan_backup_upload(protected_plan_input(evidence.clone())).unwrap();
    assert_eq!(plan.state, BackupUploadState::RemoteVerified);

    evidence.completion_recorded = true;
    let plan = plan_backup_upload(protected_plan_input(evidence)).unwrap();
    assert_eq!(plan.state, BackupUploadState::Complete);
}

#[test]
fn rejects_encrypted_upload_without_pack_protection() {
    let mut evidence = BackupUploadEvidence::none();
    evidence.pack_encrypted = true;

    assert!(matches!(
        plan_backup_upload(plan_input(evidence)),
        Err(BackupUploadError::MissingArtifactProtection)
    ));
}

#[test]
fn rejects_non_pack_protection_for_upload() {
    let mut evidence = BackupUploadEvidence::none();
    evidence.pack_encrypted = true;
    let mut input = plan_input(evidence);
    input.protection = Some(
        plan_backup_artifact_protection(BackupArtifactProtectionInput {
            artifact_kind: BackupArtifactKind::BranchManifest,
            key_ref: parse_backup_key_ref("keyring:deve/default-backup-key").unwrap(),
            encrypted: true,
            authenticated: true,
            mechanism: BackupProtectionMechanism::AeadTag,
        })
        .unwrap(),
    );

    assert!(matches!(
        plan_backup_upload(input),
        Err(BackupUploadError::ProtectionKindMismatch)
    ));
}

#[test]
fn rejects_remote_readonly_binding_for_upload() {
    let mut input = plan_input(BackupUploadEvidence::none());
    input.binding = binding(BackupBindingAccess::RemoteReadonly);

    assert!(matches!(
        plan_backup_upload(input),
        Err(BackupUploadError::ReadonlyBindingCannotUpload)
    ));
}

#[test]
fn rejects_manifest_that_does_not_match_binding() {
    let mut input = plan_input(BackupUploadEvidence::none());
    input.manifest.branch_path = "other/branches/writer-1".into();

    assert!(matches!(
        plan_backup_upload(input),
        Err(BackupUploadError::Pack(BackupPackError::BranchPathMismatch))
    ));
}

#[test]
fn rejects_out_of_order_upload_evidence() {
    let mut evidence = BackupUploadEvidence::none();
    evidence.uploaded_payload_digest = Some(digest('a'));

    assert!(matches!(
        plan_backup_upload(plan_input(evidence)),
        Err(BackupUploadError::EvidenceOutOfOrder)
    ));

    let mut evidence = BackupUploadEvidence::none();
    evidence.pack_encrypted = true;
    evidence.completion_recorded = true;

    assert!(matches!(
        plan_backup_upload(plan_input(evidence)),
        Err(BackupUploadError::EvidenceOutOfOrder)
    ));
}

#[test]
fn rejects_uploaded_or_remote_digest_mismatch() {
    let mut evidence = BackupUploadEvidence::none();
    evidence.pack_encrypted = true;
    evidence.uploaded_payload_digest = Some(digest('c'));

    assert!(matches!(
        plan_backup_upload(protected_plan_input(evidence)),
        Err(BackupUploadError::UploadedPackDigestMismatch)
    ));

    let mut evidence = BackupUploadEvidence::none();
    evidence.pack_encrypted = true;
    evidence.uploaded_payload_digest = Some(digest('a'));
    evidence.remote_manifest_payload_digest = Some(digest('c'));

    assert!(matches!(
        plan_backup_upload(protected_plan_input(evidence)),
        Err(BackupUploadError::RemoteManifestDigestMismatch)
    ));
}
