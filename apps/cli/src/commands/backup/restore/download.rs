//! plan_ref:
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-verification-contract
//!   - 06_backup#backup-provider-dispatch-contract
//!
//! Remote-readonly encrypted backup artifact download admission.
//!
//! This module performs provider download of encrypted branch manifest and pack
//! bytes and feeds them through core verification gates. It opens the branch
//! manifest required to derive typed pack refs, opens pack artifacts through
//! those refs, and admits only a remote-readonly in-memory RestoreCandidate. It
//! does not append ledger state, stage source-control changes, import branches,
//! merge branches, or touch Projection Workspaces.

use super::{RestoreCommandInput, RestoreContext, required_str};
use crate::commands::backup::provider_io::{
    BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES, BackupArtifactDownloadRequest, BackupArtifactDownloader,
    BackupArtifactKeyResolver,
};
use anyhow::bail;
use deve_core::backup::{
    BackupBindingStatus, BackupBranchManifest, BackupCommandKind, BackupDecryptedPacksInput,
    BackupDownloadedPacksInput, BackupPackArtifactRefDownloadVerifyInput,
    BackupPackArtifactRefOpenInput, BackupPackVerificationEvidence, BackupPlaintextPacksInput,
    BackupPlanInput, BackupRestoreFlowEvidence, BackupRestoreFlowInput,
    BackupRestoreResourceBudgetInput, BackupVerificationInput, RestoreAdmissionMode,
    RestoreCandidateFromVerifiedPacksInput, admit_verified_restore_candidate, backup_command_plan,
    open_backup_branch_manifest_artifact, open_backup_pack_artifact_ref,
    parse_backup_credential_ref, parse_backup_key_ref, plan_backup_restore_flow,
    validate_backup_restore_resource_budget, verify_backup_artifacts,
    verify_decrypted_backup_packs, verify_downloaded_backup_packs,
    verify_downloaded_pack_artifact_ref_and_routing, verify_plaintext_backup_packs,
};

pub(super) fn restore_download_lines(
    input: RestoreCommandInput<'_>,
    downloader: &mut dyn BackupArtifactDownloader,
    key_resolver: &mut dyn BackupArtifactKeyResolver,
    context: &RestoreContext,
) -> anyhow::Result<Vec<String>> {
    validate_download_entry(input, context)?;
    let credential_ref =
        parse_backup_credential_ref(required_str(input.credential_ref, "--credential-ref")?)?;
    let key_ref = parse_backup_key_ref(required_str(input.key_ref, "--key-ref")?)?;
    let key = key_resolver.resolve_key(&key_ref)?;

    let branch = context.locator.branch_locator(&context.writer_identity)?;
    let branch_manifest_path = branch.branch_manifest_path();
    let manifest_download = downloader.download_artifact(BackupArtifactDownloadRequest {
        locator: &context.locator,
        credential_ref: &credential_ref,
        object_path: &branch_manifest_path,
        max_bytes: BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES,
    })?;
    if !manifest_download.provider_metadata_is_diagnostic_only {
        bail!("backup provider download metadata must remain diagnostic-only");
    }
    let opened_manifest = open_backup_branch_manifest_artifact(
        deve_core::backup::BackupBranchManifestArtifactOpenInput {
            branch,
            expected_repo_id: context.repo_id,
            expected_manifest_digest: context.manifest_digest.clone(),
            key: &key,
            artifact_bytes: &manifest_download.artifact_bytes,
        },
    )?;
    let computed_manifest_digest = opened_manifest.computed_digest().clone();
    let branch_manifest = opened_manifest.into_branch_manifest();
    validate_requested_pack_digests(&context.pack_digests, &branch_manifest)?;
    let pack_count = u64::try_from(branch_manifest.packs.len()).unwrap_or(u64::MAX);
    validate_backup_restore_resource_budget(BackupRestoreResourceBudgetInput {
        pack_count,
        encrypted_bytes: 0,
        plaintext_bytes: 0,
    })?;

    let mut pack_downloaded_bytes = 0usize;
    let mut verified_packs = Vec::with_capacity(branch_manifest.packs.len());
    let mut pack_artifacts = Vec::with_capacity(branch_manifest.packs.len());
    for pack_ref in &branch_manifest.packs {
        let download = downloader.download_artifact(BackupArtifactDownloadRequest {
            locator: &context.locator,
            credential_ref: &credential_ref,
            object_path: &pack_ref.object_path,
            max_bytes: BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES,
        })?;
        if !download.provider_metadata_is_diagnostic_only {
            bail!("backup provider download metadata must remain diagnostic-only");
        }
        pack_downloaded_bytes = pack_downloaded_bytes.saturating_add(download.downloaded_bytes);
        validate_backup_restore_resource_budget(BackupRestoreResourceBudgetInput {
            pack_count,
            encrypted_bytes: pack_downloaded_bytes,
            plaintext_bytes: 0,
        })?;
        verified_packs.push(verify_downloaded_pack_artifact_ref_and_routing(
            BackupPackArtifactRefDownloadVerifyInput {
                branch_manifest: &branch_manifest,
                pack_ref,
                artifact_bytes: &download.artifact_bytes,
            },
        )?);
        pack_artifacts.push((pack_ref, download.artifact_bytes));
    }

    let downloaded = verify_downloaded_backup_packs(BackupDownloadedPacksInput {
        branch_manifest: &branch_manifest,
        verified_packs,
    })?;
    let mut opened_packs = Vec::with_capacity(pack_artifacts.len());
    let mut plaintext_bytes = 0usize;
    for (pack_ref, artifact_bytes) in pack_artifacts {
        let opened = open_backup_pack_artifact_ref(BackupPackArtifactRefOpenInput {
            branch_manifest: &branch_manifest,
            pack_ref,
            key: &key,
            artifact_bytes: &artifact_bytes,
        })?;
        plaintext_bytes = plaintext_bytes.saturating_add(opened.plaintext().len());
        validate_backup_restore_resource_budget(BackupRestoreResourceBudgetInput {
            pack_count,
            encrypted_bytes: pack_downloaded_bytes,
            plaintext_bytes,
        })?;
        opened_packs.push(opened);
    }
    let decrypted = verify_decrypted_backup_packs(BackupDecryptedPacksInput {
        downloaded_packs: &downloaded,
        opened_packs,
    })?;
    let plaintext = verify_plaintext_backup_packs(BackupPlaintextPacksInput {
        branch_manifest: &branch_manifest,
        decrypted_packs: &decrypted,
    })?;
    let manifest_verification = verify_backup_artifacts(BackupVerificationInput {
        expected_repo_id: context.repo_id,
        manifest_repo_id: context.manifest_repo_id,
        expected_manifest_digest: context.manifest_digest.clone(),
        computed_manifest_digest,
        manifest_authenticated: true,
        packs: plaintext
            .plaintext_packs()
            .iter()
            .map(|pack| BackupPackVerificationEvidence {
                pack_sequence: pack.pack_sequence(),
                expected_digest: pack.encrypted_digest().clone(),
                computed_digest: pack.encrypted_digest().clone(),
                authenticated: true,
                decrypted: true,
            })
            .collect(),
        decrypt_required: true,
    })?;
    let candidate = admit_verified_restore_candidate(RestoreCandidateFromVerifiedPacksInput {
        expected_repo_id: context.repo_id,
        manifest_verification: &manifest_verification,
        plaintext_packs: &plaintext,
        admission_mode: RestoreAdmissionMode::RemoteReadonly,
        write_gate_confirmed: false,
    })?;
    let flow = plan_backup_restore_flow(BackupRestoreFlowInput {
        expected_repo_id: context.repo_id,
        manifest_repo_id: Some(context.manifest_repo_id),
        writer_identity: context.writer_identity.clone(),
        branch_path: context.branch_path.clone(),
        manifest_digest: Some(context.manifest_digest.clone()),
        pack_digests: candidate.pack_digests.clone(),
        evidence: BackupRestoreFlowEvidence {
            remote_discovered: true,
            manifest_verified: true,
            packs_downloaded: true,
            packs_decrypted: true,
            packs_plaintext_verified: true,
            candidate_admitted: true,
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

    let mut lines = vec![
        format!(
            "backup_locator: provider={}",
            context.locator.provider.protocol()
        ),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("dry_run={}", input.dry_run),
        "artifact_io=true".to_string(),
        format!(
            "downloaded_bytes={}",
            manifest_download
                .downloaded_bytes
                .saturating_add(pack_downloaded_bytes)
        ),
        format!(
            "branch_manifest_downloaded_bytes={}",
            manifest_download.downloaded_bytes
        ),
        format!("pack_downloaded_bytes={pack_downloaded_bytes}"),
        "provider_metadata_diagnostic_only=true".to_string(),
        "manifest_verified=true".to_string(),
        format!("repo_id={}", flow.repo_id),
        format!("branch_writer={}", context.writer_identity),
        format!("branch_path={}", context.branch_path),
        format!("restore_flow_state={:?}", flow.state),
        format!("admission_mode={:?}", flow.admission_mode),
        "packs_decrypted=true".to_string(),
        "pack_plaintext_schema_verified=true".to_string(),
        "candidate_admission=created_remote_readonly".to_string(),
        format!("restore_candidate_state={:?}", candidate.state),
        format!("manifest_digest={}", context.manifest_digest.hex),
        format!("branch_manifest_object_path={branch_manifest_path}"),
        format!("pack_count={}", candidate.pack_count),
        "write_gate_confirmed=false".to_string(),
        format!("writes_local_authority={}", plan.writes_local_authority),
    ];
    for pack in downloaded.pack_refs() {
        lines.push(format!("verified_pack_sequence={}", pack.pack_sequence()));
        lines.push(format!("verified_pack_object_path={}", pack.object_path()));
        lines.push(format!(
            "verified_pack_digest={}",
            pack.payload_digest().hex
        ));
    }

    Ok(lines)
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

fn validate_requested_pack_digests(
    pack_digests: &[deve_core::backup::BackupDigest],
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
