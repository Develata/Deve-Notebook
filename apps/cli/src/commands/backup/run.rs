//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 18_backup#backup-branch-binding-contract
//!   - 18_backup#backup-pack-contract
//!   - 18_backup#backup-upload-state-machine-contract
//!   - 18_backup#backup-artifact-protection-contract
//!   - 18_backup#backup-provider-dispatch-contract
//!   - 18_backup#backup-command-output-contract
//!
//! Backup branch dry-run planning.
//!
//! This command surface validates the writable branch binding, provider refs,
//! pack manifest metadata, artifact protection evidence, and upload state. It
//! intentionally does not read ledger facts, encrypt payloads, call providers,
//! upload objects, mutate source control, or touch Projection Workspaces.

use anyhow::bail;
use deve_core::backup::{
    BackupArtifactKind, BackupArtifactProtectionInput, BackupBindingAccess, BackupBindingStatus,
    BackupBranchBindingInput, BackupCommandKind, BackupDigest, BackupLocator, BackupPackPlanInput,
    BackupPlanEffect, BackupPlanInput, BackupProtectionMechanism, BackupProviderDispatchInput,
    BackupSeqRange, BackupUploadEvidence, BackupUploadPlanInput, backup_command_plan,
    dispatch_backup_provider_adapter, parse_backup_credential_ref, parse_backup_key_ref,
    plan_backup_artifact_protection, plan_backup_branch_binding, plan_backup_pack,
    plan_backup_upload, validate_backup_branch_bindings,
};
use deve_core::models::RepoId;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub struct RunBackupCommandInput<'a> {
    pub locator: &'a str,
    pub repo_id: &'a str,
    pub branch_name: &'a str,
    pub writer_identity: &'a str,
    pub local_writer_identity: &'a str,
    pub credential_ref: &'a str,
    pub key_ref: &'a str,
    pub pack_sequence: u64,
    pub ledger_start: Option<u64>,
    pub ledger_end: Option<u64>,
    pub ledger_event_count: u64,
    pub snapshot_count: u64,
    pub payload_digest: &'a str,
    pub encrypted: bool,
    pub authenticated: bool,
    pub dry_run: bool,
}

pub fn run_backup(input: RunBackupCommandInput<'_>) -> anyhow::Result<()> {
    for line in run_backup_lines(input)? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn run_backup_lines(input: RunBackupCommandInput<'_>) -> anyhow::Result<Vec<String>> {
    if !input.dry_run {
        bail!(
            "backup run currently requires --dry-run because provider upload execution is not implemented"
        );
    }
    if !(input.encrypted && input.authenticated) {
        bail!("backup run dry-run requires --encrypted and --authenticated protection evidence");
    }

    let locator = BackupLocator::parse(input.locator)?;
    let branch = locator.branch_locator(input.writer_identity)?;
    let repo_id: RepoId = input.repo_id.parse()?;
    let credential_ref = parse_backup_credential_ref(input.credential_ref)?;
    let key_ref = parse_backup_key_ref(input.key_ref)?;
    let adapter = dispatch_backup_provider_adapter(BackupProviderDispatchInput {
        locator: locator.clone(),
        credential_ref,
        key_ref: key_ref.clone(),
    })?;
    let binding = plan_backup_branch_binding(BackupBranchBindingInput {
        repo_id,
        branch_name: input.branch_name.to_string(),
        writer_identity: branch.writer_identity.clone(),
        local_writer_identity: input.local_writer_identity.to_string(),
        branch_path: branch.branch_path.clone(),
        requested_access: BackupBindingAccess::Writable,
    })?;
    validate_backup_branch_bindings(std::slice::from_ref(&binding))?;

    let manifest = plan_backup_pack(BackupPackPlanInput {
        repo_id,
        writer_identity: branch.writer_identity,
        branch_path: branch.branch_path,
        pack_sequence: input.pack_sequence,
        ledger_seq_range: ledger_range(input)?,
        ledger_event_count: input.ledger_event_count,
        snapshot_count: input.snapshot_count,
        payload_digest: BackupDigest::sha256(input.payload_digest),
        blob_refs: Vec::new(),
    })?;
    let protection = plan_backup_artifact_protection(BackupArtifactProtectionInput {
        artifact_kind: BackupArtifactKind::Pack,
        key_ref: key_ref.clone(),
        encrypted: input.encrypted,
        authenticated: input.authenticated,
        mechanism: BackupProtectionMechanism::AeadTag,
    })?;
    let upload = plan_backup_upload(BackupUploadPlanInput {
        binding: binding.clone(),
        manifest: manifest.clone(),
        protection: Some(protection.clone()),
        evidence: BackupUploadEvidence {
            pack_encrypted: true,
            uploaded_payload_digest: None,
            remote_manifest_payload_digest: None,
            completion_recorded: false,
        },
    })?;
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::BackupBranch,
        binding_status: BackupBindingStatus::Writable,
        effect: BackupPlanEffect::RemoteUpload,
    })?;

    Ok(vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("dry_run={}", input.dry_run),
        "artifact_io=false".to_string(),
        format!("adapter_provider={}", adapter.provider.protocol()),
        format!("credential_ref={}", adapter.credential_ref.redacted()),
        format!("key_ref={}", protection.key_ref().redacted()),
        format!("repo_id={}", upload.repo_id),
        format!("branch_name={}", upload.branch_name),
        format!("writer_identity={}", upload.writer_identity),
        format!("branch_path={}", upload.branch_path),
        format!("pack_object_path={}", upload.pack_object_path),
        format!("ledger_event_count={}", manifest.ledger_event_count),
        format!("snapshot_count={}", manifest.snapshot_count),
        format!("payload_digest={}", upload.payload_digest.hex),
        format!("protection_kind={:?}", protection.artifact_kind()),
        format!("protection_mechanism={:?}", protection.mechanism()),
        format!("upload_state={:?}", upload.state),
        format!("writes_local_authority={}", plan.writes_local_authority),
    ])
}

fn ledger_range(input: RunBackupCommandInput<'_>) -> anyhow::Result<Option<BackupSeqRange>> {
    if input.ledger_event_count == 0 {
        if input.ledger_start.is_some() || input.ledger_end.is_some() {
            bail!("backup run ledger range must be omitted when --ledger-events is 0");
        }
        return Ok(None);
    }

    let (Some(start), Some(end)) = (input.ledger_start, input.ledger_end) else {
        bail!("backup run requires --ledger-start and --ledger-end when ledger events are present");
    };
    Ok(Some(BackupSeqRange { start, end }))
}
