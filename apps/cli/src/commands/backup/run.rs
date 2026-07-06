//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-pack-contract
//!   - 06_backup#backup-upload-state-machine-contract
//!   - 06_backup#backup-artifact-protection-contract
//!   - 06_backup#backup-provider-dispatch-contract
//!   - 06_backup#backup-command-output-contract
//!
//! Backup branch dry-run planning.
//!
//! This command surface validates the writable branch binding, provider refs,
//! pack manifest metadata, artifact protection evidence, and upload state. In
//! dry-run mode it does not perform provider I/O. In non-dry-run mode it only
//! uploads an explicitly supplied encrypted pack artifact after manifest/digest
//! verification. It intentionally does not read ledger facts, encrypt payloads,
//! mutate source control, or touch Projection Workspaces.

#[cfg(test)]
use super::provider_io::FailClosedBackupPackUploader;
use super::provider_io::{BackupPackUploadRequest, BackupPackUploader, RealBackupPackUploader};
use anyhow::{Context, bail};
use deve_core::backup::{
    BackupArtifactKind, BackupArtifactProtectionInput, BackupBindingAccess, BackupBindingStatus,
    BackupBranchBindingInput, BackupCommandKind, BackupDigest, BackupLocator,
    BackupPackArtifactUploadVerifyInput, BackupPackPlanInput, BackupPlanEffect, BackupPlanInput,
    BackupProtectionMechanism, BackupProviderDispatchInput, BackupSeqRange, BackupUploadEvidence,
    BackupUploadPlanInput, backup_command_plan, dispatch_backup_provider_adapter,
    parse_backup_credential_ref, parse_backup_key_ref, plan_backup_artifact_protection,
    plan_backup_branch_binding, plan_backup_pack, plan_backup_upload,
    validate_backup_branch_bindings, verify_backup_pack_artifact_for_upload,
};
use deve_core::models::RepoId;
use std::fs;
use std::path::Path;

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
    pub artifact_path: Option<&'a Path>,
    pub encrypted: bool,
    pub authenticated: bool,
    pub dry_run: bool,
}

pub fn run_backup(input: RunBackupCommandInput<'_>) -> anyhow::Result<()> {
    let mut uploader = RealBackupPackUploader;
    for line in run_backup_lines_with_uploader(input, &mut uploader)? {
        println!("{line}");
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn run_backup_lines(input: RunBackupCommandInput<'_>) -> anyhow::Result<Vec<String>> {
    let mut uploader = FailClosedBackupPackUploader;
    run_backup_lines_with_uploader(input, &mut uploader)
}

pub(crate) fn run_backup_lines_with_uploader(
    input: RunBackupCommandInput<'_>,
    uploader: &mut dyn BackupPackUploader,
) -> anyhow::Result<Vec<String>> {
    if !input.dry_run && input.artifact_path.is_none() {
        bail!("backup run provider upload requires --artifact with encrypted pack artifact bytes");
    }
    if !(input.encrypted && input.authenticated) {
        bail!("backup run requires --encrypted and --authenticated protection evidence");
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
    let artifact_bytes = match (input.dry_run, input.artifact_path) {
        (true, _) => None,
        (false, Some(path)) => Some(read_artifact_bytes(path)?),
        (false, None) => unreachable!("checked above"),
    };
    let mut evidence = BackupUploadEvidence {
        pack_encrypted: true,
        uploaded_payload_digest: None,
        remote_manifest_payload_digest: None,
        completion_recorded: false,
    };
    let upload = plan_backup_upload(BackupUploadPlanInput {
        binding: binding.clone(),
        manifest: manifest.clone(),
        protection: Some(protection.clone()),
        evidence: evidence.clone(),
    })?;
    let mut upload = upload;
    let mut uploaded_bytes = None;
    let mut remote_verified_payload_digest = None;
    let mut provider_metadata_is_diagnostic_only = None;
    if let Some(artifact_bytes) = artifact_bytes.as_deref() {
        let uploaded_digest =
            verify_backup_pack_artifact_for_upload(BackupPackArtifactUploadVerifyInput {
                manifest: &manifest,
                artifact_bytes,
            })?;
        let outcome = uploader.upload_pack(BackupPackUploadRequest {
            locator: &locator,
            credential_ref: &adapter.credential_ref,
            object_path: &upload.pack_object_path,
            artifact_bytes,
        })?;
        if !outcome.provider_metadata_is_diagnostic_only {
            bail!("backup provider upload metadata must remain diagnostic-only");
        }
        evidence.uploaded_payload_digest = Some(uploaded_digest);
        evidence.remote_manifest_payload_digest =
            Some(outcome.remote_verified_payload_digest.clone());
        upload = plan_backup_upload(BackupUploadPlanInput {
            binding: binding.clone(),
            manifest: manifest.clone(),
            protection: Some(protection.clone()),
            evidence,
        })?;
        uploaded_bytes = Some(artifact_bytes.len());
        remote_verified_payload_digest = Some(outcome.remote_verified_payload_digest);
        provider_metadata_is_diagnostic_only = Some(outcome.provider_metadata_is_diagnostic_only);
    }
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
        format!("artifact_io={}", !input.dry_run),
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
        format!(
            "uploaded_bytes={}",
            uploaded_bytes
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string())
        ),
        format!(
            "provider_metadata_diagnostic_only={}",
            provider_metadata_is_diagnostic_only
                .map(|value| value.to_string())
                .unwrap_or_else(|| "<none>".to_string())
        ),
        format!(
            "remote_verified_payload_digest={}",
            remote_verified_payload_digest
                .map(|value| value.hex)
                .unwrap_or_else(|| "<none>".to_string())
        ),
        format!("writes_local_authority={}", plan.writes_local_authority),
    ])
}

fn read_artifact_bytes(path: &Path) -> anyhow::Result<Vec<u8>> {
    let bytes = fs::read(path).with_context(|| {
        format!(
            "failed to read encrypted backup pack artifact {}",
            path.display()
        )
    })?;
    if bytes.is_empty() {
        bail!("backup run encrypted pack artifact file is empty");
    }
    Ok(bytes)
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
