//! plan_ref:
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-provider-dispatch-contract
//!
//! Remote-readonly encrypted backup pack download admission.
//!
//! This module performs provider download of encrypted pack bytes and feeds the
//! result through core verification gates. It does not decrypt, create restore
//! candidates, append ledger state, stage source-control changes, or touch
//! Projection Workspaces.

use super::{RestoreCommandInput, RestoreContext, ledger_range, required_str, required_u64};
use crate::commands::backup::provider_io::{
    BACKUP_PACK_MAX_DOWNLOAD_BYTES, BackupPackDownloadRequest, BackupPackDownloader,
};
use anyhow::bail;
use deve_core::backup::{
    BACKUP_BRANCH_MANIFEST_FORMAT_VERSION, BackupBindingStatus, BackupBranchManifestInput,
    BackupBranchManifestPackRef, BackupCommandKind, BackupDownloadedPacksInput,
    BackupPackArtifactDownloadVerifyInput, BackupPackPlanInput, BackupPlanInput,
    BackupRestoreFlowEvidence, BackupRestoreFlowInput, RestoreAdmissionMode, backup_command_plan,
    parse_backup_credential_ref, plan_backup_pack, plan_backup_restore_flow,
    validate_backup_branch_manifest, verify_downloaded_backup_packs,
    verify_downloaded_pack_artifact_digest_and_routing,
};

pub(super) fn restore_download_lines(
    input: RestoreCommandInput<'_>,
    downloader: &mut dyn BackupPackDownloader,
    context: &RestoreContext,
) -> anyhow::Result<Vec<String>> {
    validate_download_entry(input, context)?;
    let credential_ref =
        parse_backup_credential_ref(required_str(input.credential_ref, "--credential-ref")?)?;
    let pack_sequence = required_u64(input.pack_sequence, "--pack-sequence")?;
    let ledger_event_count = required_u64(input.ledger_event_count, "--ledger-events")?;
    let snapshot_count = required_u64(input.snapshot_count, "--snapshot-count")?;
    let ledger_seq_range = ledger_range(input.ledger_start, input.ledger_end, ledger_event_count)?;
    let manifest = plan_backup_pack(BackupPackPlanInput {
        repo_id: context.repo_id,
        writer_identity: context.writer_identity.clone(),
        branch_path: context.branch_path.clone(),
        pack_sequence,
        ledger_seq_range,
        ledger_event_count,
        snapshot_count,
        payload_digest: context.pack_digests[0].clone(),
        blob_refs: Vec::new(),
    })?;
    let branch_manifest = validate_backup_branch_manifest(BackupBranchManifestInput {
        branch: context.locator.branch_locator(&context.writer_identity)?,
        expected_repo_id: context.repo_id,
        manifest_repo_id: context.manifest_repo_id,
        manifest_writer_identity: context.writer_identity.clone(),
        manifest_branch_path: context.branch_path.clone(),
        format_version: BACKUP_BRANCH_MANIFEST_FORMAT_VERSION,
        packs: vec![BackupBranchManifestPackRef {
            pack_sequence: manifest.pack_sequence,
            object_path: manifest.pack_object_path(),
            payload_digest: manifest.payload_digest.clone(),
        }],
    })?;
    let expected_pack = &branch_manifest.packs[0];
    let download = downloader.download_pack(BackupPackDownloadRequest {
        locator: &context.locator,
        credential_ref: &credential_ref,
        object_path: &expected_pack.object_path,
        max_bytes: BACKUP_PACK_MAX_DOWNLOAD_BYTES,
    })?;
    if !download.provider_metadata_is_diagnostic_only {
        bail!("backup provider download metadata must remain diagnostic-only");
    }
    let verified = verify_downloaded_pack_artifact_digest_and_routing(
        BackupPackArtifactDownloadVerifyInput {
            manifest: &manifest,
            artifact_bytes: &download.artifact_bytes,
        },
    )?;

    let downloaded = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
        branch_manifest: &branch_manifest,
        verified_packs: vec![verified.clone()],
    })?;
    let flow = plan_backup_restore_flow(BackupRestoreFlowInput {
        expected_repo_id: context.repo_id,
        manifest_repo_id: Some(context.manifest_repo_id),
        writer_identity: context.writer_identity.clone(),
        branch_path: context.branch_path.clone(),
        manifest_digest: Some(context.manifest_digest.clone()),
        pack_digests: downloaded.pack_digests().to_vec(),
        evidence: BackupRestoreFlowEvidence {
            remote_discovered: true,
            manifest_verified: true,
            packs_downloaded: true,
            packs_decrypted: false,
            candidate_admitted: false,
        },
        admission_mode: context.admission_mode,
        write_gate_confirmed: false,
        local_ledger_append_requested: false,
    })?;
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::RestoreBackup,
        binding_status: BackupBindingStatus::Unbound,
        effect: context.effect,
    })?;

    Ok(vec![
        format!(
            "backup_locator: provider={}",
            context.locator.provider.protocol()
        ),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("dry_run={}", input.dry_run),
        "artifact_io=true".to_string(),
        format!("downloaded_bytes={}", download.downloaded_bytes),
        format!(
            "provider_metadata_diagnostic_only={}",
            download.provider_metadata_is_diagnostic_only
        ),
        format!("repo_id={}", flow.repo_id),
        format!("branch_writer={}", context.writer_identity),
        format!("branch_path={}", context.branch_path),
        format!("restore_flow_state={:?}", flow.state),
        format!("admission_mode={:?}", flow.admission_mode),
        "candidate_admission=not_created_download_verify_only".to_string(),
        format!("manifest_digest={}", context.manifest_digest.hex),
        format!("pack_count={}", downloaded.pack_count()),
        format!("verified_pack_sequence={}", verified.pack_sequence()),
        format!("verified_pack_object_path={}", verified.object_path()),
        format!("verified_pack_digest={}", verified.computed_digest().hex),
        "write_gate_confirmed=false".to_string(),
        format!("writes_local_authority={}", plan.writes_local_authority),
    ])
}

fn validate_download_entry(
    input: RestoreCommandInput<'_>,
    context: &RestoreContext,
) -> anyhow::Result<()> {
    if context.admission_mode != RestoreAdmissionMode::RemoteReadonly {
        bail!(
            "backup restore explicit import or merge remains fail-closed until RestoreCandidate import/merge authority is implemented"
        );
    }
    if !input.manifest_verified {
        bail!("backup restore provider download requires --manifest-verified evidence");
    }
    if input.packs_decrypted {
        bail!("backup restore provider download stops before --packs-decrypted evidence");
    }
    if context.pack_digests.len() != 1 {
        bail!("backup restore provider download currently requires exactly one --pack-digest");
    }

    plan_backup_restore_flow(BackupRestoreFlowInput {
        expected_repo_id: context.repo_id,
        manifest_repo_id: Some(context.manifest_repo_id),
        writer_identity: context.writer_identity.clone(),
        branch_path: context.branch_path.clone(),
        manifest_digest: Some(context.manifest_digest.clone()),
        pack_digests: context.pack_digests.clone(),
        evidence: BackupRestoreFlowEvidence {
            remote_discovered: true,
            manifest_verified: true,
            packs_downloaded: false,
            packs_decrypted: false,
            candidate_admitted: false,
        },
        admission_mode: context.admission_mode,
        write_gate_confirmed: false,
        local_ledger_append_requested: false,
    })?;
    Ok(())
}
