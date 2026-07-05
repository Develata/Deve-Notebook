//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 06_backup#backup-restore-state-machine-contract
//!   - 06_backup#backup-restore-candidate-contract
//!   - 06_backup#backup-command-output-contract
//!
//! Backup restore flow planning and encrypted artifact download admission.
//!
//! This command surface validates caller-supplied flow metadata for dry-runs and
//! can perform a remote-readonly provider download of encrypted branch manifest
//! and pack bytes. Manifest bytes are opened through core verification before
//! pack refs are trusted; pack bytes are opened through typed manifest refs and
//! admitted as an in-memory remote-readonly RestoreCandidate. This command does
//! not import, merge, append ledger state, or touch Projection Workspaces.

mod download;

use super::provider_io::{
    BackupArtifactDownloader, BackupArtifactKeyResolver, EnvBackupArtifactKeyResolver,
    RealBackupArtifactDownloader,
};
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
    pub credential_ref: Option<&'a str>,
    pub key_ref: Option<&'a str>,
    pub pack_sequence: Option<u64>,
    pub ledger_start: Option<u64>,
    pub ledger_end: Option<u64>,
    pub ledger_event_count: Option<u64>,
    pub snapshot_count: Option<u64>,
}

pub fn restore(input: RestoreCommandInput<'_>) -> anyhow::Result<()> {
    let mut downloader = RealBackupArtifactDownloader;
    let mut key_resolver = EnvBackupArtifactKeyResolver;
    for line in restore_lines_with_runtime(input, &mut downloader, &mut key_resolver)? {
        println!("{line}");
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn restore_lines(input: RestoreCommandInput<'_>) -> anyhow::Result<Vec<String>> {
    let mut downloader = FailClosedRestoreDownloader;
    let mut key_resolver = FailClosedRestoreKeyResolver;
    restore_lines_with_runtime(input, &mut downloader, &mut key_resolver)
}

pub(crate) fn restore_lines_with_runtime(
    input: RestoreCommandInput<'_>,
    downloader: &mut dyn BackupArtifactDownloader,
    key_resolver: &mut dyn BackupArtifactKeyResolver,
) -> anyhow::Result<Vec<String>> {
    let context = RestoreContext::parse(input)?;
    if requires_write_gate(context.admission_mode) && !input.write_gate {
        bail!("backup restore explicit import or merge requires an explicit write gate");
    }

    if input.dry_run {
        return restore_dry_run_lines(input, &context);
    }

    download::restore_download_lines(input, downloader, key_resolver, &context)
}

fn restore_dry_run_lines(
    input: RestoreCommandInput<'_>,
    context: &RestoreContext,
) -> anyhow::Result<Vec<String>> {
    if !(input.manifest_verified && input.packs_downloaded && input.packs_decrypted) {
        bail!(
            "backup restore dry-run requires --manifest-verified, --packs-downloaded, and --packs-decrypted evidence"
        );
    }

    let flow = plan_backup_restore_flow(BackupRestoreFlowInput {
        expected_repo_id: context.repo_id,
        manifest_repo_id: Some(context.manifest_repo_id),
        writer_identity: context.writer_identity.clone(),
        branch_path: context.branch_path.clone(),
        manifest_digest: Some(context.manifest_digest.clone()),
        pack_digests: context.pack_digests.clone(),
        evidence: BackupRestoreFlowEvidence {
            remote_discovered: true,
            manifest_verified: input.manifest_verified,
            packs_downloaded: input.packs_downloaded,
            packs_decrypted: input.packs_decrypted,
            candidate_admitted: false,
        },
        admission_mode: context.admission_mode,
        write_gate_confirmed: input.write_gate,
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
        "artifact_io=false".to_string(),
        format!("repo_id={}", flow.repo_id),
        format!("branch_writer={}", context.writer_identity),
        format!("branch_path={}", context.branch_path),
        format!("restore_flow_state={:?}", flow.state),
        format!("admission_mode={:?}", flow.admission_mode),
        "candidate_admission=typed_verification_and_decrypted_evidence_required".to_string(),
        format!("manifest_digest={}", context.manifest_digest.hex),
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

pub(super) struct RestoreContext {
    pub locator: BackupLocator,
    pub writer_identity: String,
    pub branch_path: String,
    pub repo_id: RepoId,
    pub manifest_repo_id: RepoId,
    pub manifest_digest: BackupDigest,
    pub pack_digests: Vec<BackupDigest>,
    pub admission_mode: RestoreAdmissionMode,
    pub effect: BackupPlanEffect,
}

impl RestoreContext {
    fn parse(input: RestoreCommandInput<'_>) -> anyhow::Result<Self> {
        let locator = BackupLocator::parse(input.locator)?;
        let branch = locator.branch_locator(input.branch)?;
        let repo_id: RepoId = input.repo_id.parse()?;
        let manifest_repo_id: RepoId = input.manifest_repo_id.parse()?;
        let manifest_digest = BackupDigest::sha256(input.manifest_digest);
        let pack_digests = parse_pack_digests(input.pack_digests);
        let (admission_mode, effect) = parse_mode(input.mode)?;

        Ok(Self {
            locator,
            writer_identity: branch.writer_identity,
            branch_path: branch.branch_path,
            repo_id,
            manifest_repo_id,
            manifest_digest,
            pack_digests,
            admission_mode,
            effect,
        })
    }
}

pub(super) fn required_str<'a>(value: Option<&'a str>, flag: &str) -> anyhow::Result<&'a str> {
    value.ok_or_else(|| anyhow::anyhow!("backup restore provider download requires {flag}"))
}

#[cfg(test)]
struct FailClosedRestoreDownloader;

#[cfg(test)]
impl BackupArtifactDownloader for FailClosedRestoreDownloader {
    fn download_artifact(
        &mut self,
        _request: super::provider_io::BackupArtifactDownloadRequest<'_>,
    ) -> anyhow::Result<super::provider_io::BackupArtifactDownloadOutcome> {
        bail!("backup provider download is unavailable in this execution path")
    }
}

#[cfg(test)]
struct FailClosedRestoreKeyResolver;

#[cfg(test)]
impl BackupArtifactKeyResolver for FailClosedRestoreKeyResolver {
    fn resolve_key(
        &mut self,
        _key_ref: &deve_core::backup::BackupSecretRef,
    ) -> anyhow::Result<deve_core::backup::BackupArtifactKey> {
        bail!("backup key resolution is unavailable in this execution path")
    }
}
