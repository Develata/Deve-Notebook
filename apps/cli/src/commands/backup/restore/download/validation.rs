//! plan_ref:
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-provider-dispatch-contract
//!
//! Provider download admission checks for backup restore.

use super::super::{RestoreCommandInput, RestoreContext};
use anyhow::bail;
use deve_core::backup::{
    BackupBranchManifest, BackupDigest, BackupRestoreFlowEvidence, BackupRestoreFlowInput,
    RestoreAdmissionMode, plan_backup_restore_flow,
};

pub(super) fn validate_download_entry(
    input: RestoreCommandInput<'_>,
    context: &RestoreContext,
) -> anyhow::Result<()> {
    if context.admission_mode != RestoreAdmissionMode::RemoteReadonly {
        bail!(
            "backup restore explicit import or merge remains fail-closed until RestoreCandidate import/merge authority is implemented"
        );
    }
    if input.manifest_verified || input.packs_downloaded || input.packs_decrypted {
        bail!(
            "backup restore provider download derives manifest/download evidence from remote artifacts; do not pass precomputed evidence flags"
        );
    }
    if input.pack_sequence.is_some()
        || input.ledger_start.is_some()
        || input.ledger_end.is_some()
        || input.ledger_event_count.is_some()
        || input.snapshot_count.is_some()
    {
        bail!(
            "backup restore provider download derives pack refs from branch.manifest.enc; remove pack sequence and ledger metadata flags"
        );
    }
    if !context.manifest_digest.is_valid_sha256() {
        bail!("backup restore provider download requires a valid --manifest-digest");
    }
    if context.manifest_repo_id != context.repo_id {
        bail!("backup restore manifest repo id does not match expected repo id");
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
            manifest_verified: false,
            packs_downloaded: false,
            packs_decrypted: false,
            packs_plaintext_verified: false,
            candidate_admitted: false,
        },
        admission_mode: context.admission_mode,
        write_gate_confirmed: false,
        local_ledger_append_requested: false,
    })?;
    Ok(())
}

pub(super) fn validate_requested_pack_digests(
    pack_digests: &[BackupDigest],
    branch_manifest: &BackupBranchManifest,
) -> anyhow::Result<()> {
    if pack_digests.is_empty() {
        return Ok(());
    }
    if pack_digests.len() != branch_manifest.packs.len() {
        bail!("backup restore --pack-digest count must match branch manifest pack refs");
    }
    for (provided, expected) in pack_digests.iter().zip(&branch_manifest.packs) {
        if !expected.payload_digest.same_sha256(provided) {
            bail!("backup restore --pack-digest does not match branch manifest pack ref");
        }
    }
    Ok(())
}
