//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-restore-candidate-contract
//!   - 06_backup#backup-command-output-contract
//!
//! Backup restore dry-run flow planning.
//!
//! This command surface validates caller-supplied flow metadata without
//! admitting a RestoreCandidate. Candidate admission requires typed manifest
//! verification and decrypted pack evidence in core. This command intentionally
//! does not download, decrypt, import, merge, append ledger state, or touch
//! Projection Workspaces.

use anyhow::bail;
use deve_core::backup::{
    BackupBindingStatus, BackupCommandKind, BackupDigest, BackupLocator, BackupPlanEffect,
    BackupPlanInput, BackupRestoreFlowEvidence, BackupRestoreFlowInput, RestoreAdmissionMode,
    backup_command_plan, plan_backup_restore_flow,
};
use deve_core::models::RepoId;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub struct RestoreCommandInput<'a> {
    pub locator: &'a str,
    pub repo_id: &'a str,
    pub manifest_repo_id: &'a str,
    pub branch: &'a str,
    pub manifest_digest: &'a str,
    pub pack_digests: &'a [String],
    pub mode: &'a str,
    pub write_gate: bool,
    pub manifest_verified: bool,
    pub packs_downloaded: bool,
    pub packs_decrypted: bool,
    pub dry_run: bool,
}

pub fn restore(input: RestoreCommandInput<'_>) -> anyhow::Result<()> {
    for line in restore_lines(input)? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn restore_lines(input: RestoreCommandInput<'_>) -> anyhow::Result<Vec<String>> {
    if !input.dry_run {
        bail!(
            "backup restore currently requires --dry-run because provider IO and import/merge execution are not implemented"
        );
    }
    if !(input.manifest_verified && input.packs_downloaded && input.packs_decrypted) {
        bail!(
            "backup restore dry-run requires --manifest-verified, --packs-downloaded, and --packs-decrypted evidence"
        );
    }

    let locator = BackupLocator::parse(input.locator)?;
    let branch = locator.branch_locator(input.branch)?;
    let repo_id: RepoId = input.repo_id.parse()?;
    let manifest_repo_id: RepoId = input.manifest_repo_id.parse()?;
    let manifest_digest = BackupDigest::sha256(input.manifest_digest);
    let pack_digests = parse_pack_digests(input.pack_digests);
    let (admission_mode, effect) = parse_mode(input.mode)?;
    if requires_write_gate(admission_mode) && !input.write_gate {
        bail!("backup restore explicit import or merge requires an explicit write gate");
    }

    let flow = plan_backup_restore_flow(BackupRestoreFlowInput {
        expected_repo_id: repo_id,
        manifest_repo_id: Some(manifest_repo_id),
        writer_identity: branch.writer_identity.clone(),
        branch_path: branch.branch_path.clone(),
        manifest_digest: Some(manifest_digest.clone()),
        pack_digests: pack_digests.clone(),
        evidence: BackupRestoreFlowEvidence {
            remote_discovered: true,
            manifest_verified: input.manifest_verified,
            packs_downloaded: input.packs_downloaded,
            packs_decrypted: input.packs_decrypted,
            candidate_admitted: false,
        },
        admission_mode,
        write_gate_confirmed: input.write_gate,
        local_ledger_append_requested: false,
    })?;
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::RestoreBackup,
        binding_status: BackupBindingStatus::Unbound,
        effect,
    })?;

    Ok(vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("dry_run={}", input.dry_run),
        "artifact_io=false".to_string(),
        format!("repo_id={}", flow.repo_id),
        format!("branch_writer={}", flow.writer_identity),
        format!("branch_path={}", flow.branch_path),
        format!("restore_flow_state={:?}", flow.state),
        format!("admission_mode={:?}", flow.admission_mode),
        "candidate_admission=typed_verification_and_decrypted_evidence_required".to_string(),
        format!("manifest_digest={}", manifest_digest.hex),
        format!("pack_count={}", flow.pack_count),
        format!("write_gate_confirmed={}", input.write_gate),
        format!("writes_local_authority={}", plan.writes_local_authority),
    ])
}

fn parse_pack_digests(pack_digests: &[String]) -> Vec<BackupDigest> {
    pack_digests
        .iter()
        .map(|digest| BackupDigest::sha256(digest.clone()))
        .collect()
}

fn parse_mode(mode: &str) -> anyhow::Result<(RestoreAdmissionMode, BackupPlanEffect)> {
    match mode {
        "remote-readonly" => Ok((
            RestoreAdmissionMode::RemoteReadonly,
            BackupPlanEffect::RemoteDownload,
        )),
        "explicit-import" => Ok((
            RestoreAdmissionMode::ExplicitImport,
            BackupPlanEffect::ExplicitImport,
        )),
        "explicit-merge" => Ok((
            RestoreAdmissionMode::ExplicitMerge,
            BackupPlanEffect::ExplicitMerge,
        )),
        _ => {
            bail!("backup restore mode must be remote-readonly, explicit-import, or explicit-merge")
        }
    }
}

fn requires_write_gate(mode: RestoreAdmissionMode) -> bool {
    matches!(
        mode,
        RestoreAdmissionMode::ExplicitImport | RestoreAdmissionMode::ExplicitMerge
    )
}
