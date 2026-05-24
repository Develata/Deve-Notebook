//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 18_backup#backup-locator-contract
//!   - 18_backup#backup-provider-dispatch-contract
//!   - 18_backup#backup-secret-ref-contract
//!   - 18_backup#backup-command-output-contract
//!
//! Read-only backup locator and provider adapter command surface.

use anyhow::bail;
use deve_core::backup::{
    BackupBindingStatus, BackupBranchDiscoveryInput, BackupCommandKind, BackupLocator,
    BackupPlanEffect, BackupPlanInput, BackupRemoteLayoutInput, BackupRemoteObject,
    backup_command_plan, discover_backup_branches, dispatch_backup_provider_adapter,
    inspect_backup_remote_layout, parse_backup_credential_ref, parse_backup_key_ref,
};

pub fn inspect(
    locator: &str,
    branch: Option<&str>,
    credential_ref: Option<&str>,
    key_ref: Option<&str>,
) -> anyhow::Result<()> {
    for line in inspect_lines(locator, branch, credential_ref, key_ref)? {
        println!("{line}");
    }
    Ok(())
}

pub fn list(locator: &str, object_paths: &[String]) -> anyhow::Result<()> {
    for line in list_lines(locator, object_paths)? {
        println!("{line}");
    }
    Ok(())
}

pub fn verify(
    locator: &str,
    branch: &str,
    object_paths: &[String],
    expected_pack_paths: &[String],
) -> anyhow::Result<()> {
    for line in verify_lines(locator, branch, object_paths, expected_pack_paths)? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn inspect_lines(
    locator: &str,
    branch: Option<&str>,
    credential_ref: Option<&str>,
    key_ref: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    let locator = BackupLocator::parse(locator)?;
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::InspectBackupTarget,
        binding_status: BackupBindingStatus::Unbound,
        effect: BackupPlanEffect::InspectOnly,
    })?;
    let mut lines = vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!(
            "endpoint={}",
            locator.endpoint.as_deref().unwrap_or("<provider-default>")
        ),
        format!("namespace={}", locator.namespace),
        format!("repo_root_path={}", locator.repo_root_path),
    ];

    if let Some(writer_identity) = branch {
        let branch = locator.branch_locator(writer_identity)?;
        lines.push(format!("branch_writer={}", branch.writer_identity));
        lines.push(format!("branch_path={}", branch.branch_path));
        lines.push(format!("branch_manifest={}", branch.branch_manifest_path()));
        lines.push(format!("pack_prefix={}", branch.pack_prefix()));
    }

    append_provider_adapter_lines(&mut lines, &locator, credential_ref, key_ref)?;

    Ok(lines)
}

pub(crate) fn list_lines(locator: &str, object_paths: &[String]) -> anyhow::Result<Vec<String>> {
    let locator = BackupLocator::parse(locator)?;
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::ListBackupBranches,
        binding_status: BackupBindingStatus::Unbound,
        effect: BackupPlanEffect::InspectOnly,
    })?;
    let report = discover_backup_branches(BackupBranchDiscoveryInput {
        repo_locator: locator.clone(),
        objects: object_paths
            .iter()
            .map(|path| BackupRemoteObject {
                path: path.clone(),
                metadata: None,
            })
            .collect(),
    });

    let mut lines = vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("repo_manifest={}", report.repo_manifest_path),
        format!("observed_object_count={}", report.observed_object_count),
        format!("branch_count={}", report.branches.len()),
    ];

    for branch in report.branches {
        lines.push(format!(
            "branch writer={} path={} manifest={} pack_prefix={}",
            branch.writer_identity,
            branch.branch_path,
            branch.branch_manifest_path,
            branch.pack_prefix
        ));
    }
    for diagnostic in report.diagnostics {
        lines.push(format_discovery_diagnostic(diagnostic));
    }

    Ok(lines)
}

pub(crate) fn verify_lines(
    locator: &str,
    branch: &str,
    object_paths: &[String],
    expected_pack_paths: &[String],
) -> anyhow::Result<Vec<String>> {
    let locator = BackupLocator::parse(locator)?;
    let branch = locator.branch_locator(branch)?;
    let branch_writer = branch.writer_identity.clone();
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::VerifyBackupTarget,
        binding_status: BackupBindingStatus::Unbound,
        effect: BackupPlanEffect::RemoteVerify,
    })?;
    let report = inspect_backup_remote_layout(BackupRemoteLayoutInput {
        branch,
        objects: object_paths
            .iter()
            .map(|path| BackupRemoteObject {
                path: path.clone(),
                metadata: None,
            })
            .collect(),
        expected_pack_object_paths: expected_pack_paths.to_vec(),
    })?;

    let mut lines = vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("branch_writer={branch_writer}"),
        format!("repo_manifest={}", report.repo_manifest_path),
        format!("branch_manifest={}", report.branch_manifest_path),
        format!("pack_prefix={}", report.pack_prefix),
        format!("observed_object_count={}", report.observed_object_count),
        format!("expected_pack_count={}", expected_pack_paths.len()),
        format!("layout_healthy={}", report.is_healthy()),
    ];

    for diagnostic in report.diagnostics {
        lines.push(format_layout_diagnostic(diagnostic));
    }

    Ok(lines)
}

fn append_provider_adapter_lines(
    lines: &mut Vec<String>,
    locator: &BackupLocator,
    credential_ref: Option<&str>,
    key_ref: Option<&str>,
) -> anyhow::Result<()> {
    let (Some(credential_ref), Some(key_ref)) = (credential_ref, key_ref) else {
        if credential_ref.is_some() || key_ref.is_some() {
            bail!(
                "backup inspect provider adapter plan requires both --credential-ref and --key-ref"
            );
        }
        return Ok(());
    };

    let adapter =
        dispatch_backup_provider_adapter(deve_core::backup::BackupProviderDispatchInput {
            locator: locator.clone(),
            credential_ref: parse_backup_credential_ref(credential_ref)?,
            key_ref: parse_backup_key_ref(key_ref)?,
        })?;

    lines.push(format!("adapter_provider={}", adapter.provider.protocol()));
    lines.push(format!(
        "credential_ref={}",
        adapter.credential_ref.redacted()
    ));
    lines.push(format!("key_ref={}", adapter.key_ref.redacted()));
    lines.push(format!(
        "provider_metadata_diagnostic_only={}",
        adapter.provider_metadata_is_diagnostic_only
    ));
    Ok(())
}

fn format_discovery_diagnostic(
    diagnostic: deve_core::backup::BackupBranchDiscoveryDiagnostic,
) -> String {
    let mut line = format!("diagnostic kind={:?}", diagnostic.kind);
    if let Some(path) = diagnostic.path {
        line.push_str(&format!(" path={path}"));
    }
    if let Some(detail) = diagnostic.detail {
        line.push_str(&format!(" detail={detail}"));
    }
    line
}

fn format_layout_diagnostic(diagnostic: deve_core::backup::BackupRemoteLayoutDiagnostic) -> String {
    let mut line = format!("diagnostic kind={:?}", diagnostic.kind);
    if let Some(path) = diagnostic.path {
        line.push_str(&format!(" path={path}"));
    }
    if let Some(detail) = diagnostic.detail {
        line.push_str(&format!(" detail={detail}"));
    }
    line
}

#[cfg(test)]
mod tests;
