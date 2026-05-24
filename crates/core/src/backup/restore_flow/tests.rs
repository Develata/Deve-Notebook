use super::{
    BackupDigest, BackupRestoreFlowError, BackupRestoreFlowEvidence, BackupRestoreFlowInput,
    BackupRestoreFlowState, RestoreAdmissionMode, plan_backup_restore_flow,
};
use crate::backup::BackupLocator;

fn digest(seed: char) -> BackupDigest {
    BackupDigest::sha256(seed.to_string().repeat(64))
}

fn input(evidence: BackupRestoreFlowEvidence) -> BackupRestoreFlowInput {
    let branch = BackupLocator::parse("s3://bucket-name/deve/")
        .unwrap()
        .branch_locator("writer-1")
        .unwrap();

    BackupRestoreFlowInput {
        expected_repo_id: uuid::Uuid::from_u128(42),
        manifest_repo_id: None,
        writer_identity: branch.writer_identity,
        branch_path: branch.branch_path,
        manifest_digest: None,
        pack_digests: Vec::new(),
        evidence,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
        local_ledger_append_requested: false,
    }
}

fn verified_input(evidence: BackupRestoreFlowEvidence) -> BackupRestoreFlowInput {
    let mut input = input(evidence);
    input.manifest_repo_id = Some(input.expected_repo_id);
    input.manifest_digest = Some(digest('a'));
    input.pack_digests = vec![digest('b'), digest('c')];
    input
}

#[test]
fn plans_remote_discovered_state_without_write_effects() {
    let plan =
        plan_backup_restore_flow(input(BackupRestoreFlowEvidence::remote_discovered())).unwrap();

    assert_eq!(plan.writer_identity, "writer-1");
    assert_eq!(plan.branch_path, "deve/branches/writer-1");
    assert_eq!(plan.pack_count, 0);
    assert_eq!(plan.state, BackupRestoreFlowState::RemoteDiscovered);
}

#[test]
fn progresses_to_restore_candidate_after_verify_download_decrypt() {
    let evidence = BackupRestoreFlowEvidence {
        remote_discovered: true,
        manifest_verified: true,
        packs_downloaded: true,
        packs_decrypted: true,
        candidate_admitted: true,
    };

    let plan = plan_backup_restore_flow(verified_input(evidence)).unwrap();

    assert_eq!(plan.pack_count, 2);
    assert_eq!(plan.state, BackupRestoreFlowState::RestoreCandidate);
}

#[test]
fn rejects_out_of_order_restore_evidence() {
    let evidence = BackupRestoreFlowEvidence {
        remote_discovered: true,
        manifest_verified: false,
        packs_downloaded: false,
        packs_decrypted: true,
        candidate_admitted: false,
    };

    assert!(matches!(
        plan_backup_restore_flow(input(evidence)),
        Err(BackupRestoreFlowError::EvidenceOutOfOrder)
    ));

    let mut evidence = BackupRestoreFlowEvidence::remote_discovered();
    evidence.remote_discovered = false;
    assert!(matches!(
        plan_backup_restore_flow(input(evidence)),
        Err(BackupRestoreFlowError::RemoteNotDiscovered)
    ));
}

#[test]
fn rejects_manifest_repo_or_digest_mismatch() {
    let evidence = BackupRestoreFlowEvidence {
        remote_discovered: true,
        manifest_verified: true,
        packs_downloaded: false,
        packs_decrypted: false,
        candidate_admitted: false,
    };
    let mut restore_input = verified_input(evidence);
    restore_input.manifest_repo_id = Some(uuid::Uuid::from_u128(7));

    assert!(matches!(
        plan_backup_restore_flow(restore_input),
        Err(BackupRestoreFlowError::RepoIdMismatch)
    ));

    let mut restore_input = verified_input(evidence);
    restore_input.manifest_digest = Some(BackupDigest::sha256("not-hex"));

    assert!(matches!(
        plan_backup_restore_flow(restore_input),
        Err(BackupRestoreFlowError::InvalidDigest)
    ));
}

#[test]
fn rejects_empty_pack_download_after_download_phase() {
    let evidence = BackupRestoreFlowEvidence {
        remote_discovered: true,
        manifest_verified: true,
        packs_downloaded: true,
        packs_decrypted: false,
        candidate_admitted: false,
    };
    let mut restore_input = verified_input(evidence);
    restore_input.pack_digests.clear();

    assert!(matches!(
        plan_backup_restore_flow(restore_input),
        Err(BackupRestoreFlowError::EmptyPackDownload)
    ));

    let mut restore_input = verified_input(evidence);
    restore_input.pack_digests[1] = restore_input.pack_digests[0].clone();

    assert!(matches!(
        plan_backup_restore_flow(restore_input),
        Err(BackupRestoreFlowError::DuplicatePackDigest)
    ));
}

#[test]
fn blocks_ledger_append_until_explicit_write_gate() {
    let evidence = BackupRestoreFlowEvidence {
        remote_discovered: true,
        manifest_verified: true,
        packs_downloaded: true,
        packs_decrypted: true,
        candidate_admitted: false,
    };
    let mut restore_input = verified_input(evidence);
    restore_input.local_ledger_append_requested = true;

    assert!(matches!(
        plan_backup_restore_flow(restore_input),
        Err(BackupRestoreFlowError::LocalLedgerAppendForbidden)
    ));

    let evidence = BackupRestoreFlowEvidence {
        candidate_admitted: true,
        ..evidence
    };
    let mut restore_input = verified_input(evidence);
    restore_input.admission_mode = RestoreAdmissionMode::ExplicitImport;

    assert!(matches!(
        plan_backup_restore_flow(restore_input.clone()),
        Err(BackupRestoreFlowError::WriteGateRequired)
    ));

    restore_input.write_gate_confirmed = true;
    restore_input.local_ledger_append_requested = true;
    let plan = plan_backup_restore_flow(restore_input).unwrap();
    assert_eq!(plan.state, BackupRestoreFlowState::RestoreCandidate);
}
