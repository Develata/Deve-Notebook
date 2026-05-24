//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 18_backup#backup-branch-binding-contract
//!   - 18_backup#backup-command-output-contract
//!
//! Backup bind command dry-run planning.
//!
//! This command surface validates branch backup binding inputs and prints the
//! planned binding. It intentionally does not persist binding state, contact
//! providers, write ledger state, or touch Projection Workspaces.

use anyhow::bail;
use deve_core::backup::{
    BackupBindingAccess, BackupBindingStatus, BackupBranchBindingInput, BackupCommandKind,
    BackupLocator, BackupPlanEffect, BackupPlanInput, backup_command_plan,
    plan_backup_branch_binding, validate_backup_branch_bindings,
};
use deve_core::models::RepoId;

pub fn bind(
    locator: &str,
    repo_id: &str,
    branch_name: &str,
    writer_identity: &str,
    local_writer_identity: &str,
    access: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    for line in bind_lines(
        locator,
        repo_id,
        branch_name,
        writer_identity,
        local_writer_identity,
        access,
        dry_run,
    )? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn bind_lines(
    locator: &str,
    repo_id: &str,
    branch_name: &str,
    writer_identity: &str,
    local_writer_identity: &str,
    access: &str,
    dry_run: bool,
) -> anyhow::Result<Vec<String>> {
    if !dry_run {
        bail!(
            "backup bind currently requires --dry-run because binding persistence is not implemented"
        );
    }

    let locator = BackupLocator::parse(locator)?;
    let branch = locator.branch_locator(writer_identity)?;
    let repo_id: RepoId = repo_id.parse()?;
    let access = parse_access(access)?;
    let binding = plan_backup_branch_binding(BackupBranchBindingInput {
        repo_id,
        branch_name: branch_name.to_string(),
        writer_identity: branch.writer_identity,
        local_writer_identity: local_writer_identity.to_string(),
        branch_path: branch.branch_path,
        requested_access: access,
    })?;
    validate_backup_branch_bindings(std::slice::from_ref(&binding))?;
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::BindBackupTarget,
        binding_status: BackupBindingStatus::Unbound,
        effect: BackupPlanEffect::BindingMutation,
    })?;

    Ok(vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("dry_run={dry_run}"),
        format!("repo_id={}", binding.repo_id),
        format!("branch_name={}", binding.branch_name),
        format!("writer_identity={}", binding.writer_identity),
        format!("branch_path={}", binding.branch_path),
        format!("access={:?}", binding.access),
        format!("writes_local_authority={}", plan.writes_local_authority),
    ])
}

fn parse_access(access: &str) -> anyhow::Result<BackupBindingAccess> {
    match access {
        "writable" => Ok(BackupBindingAccess::Writable),
        "remote-readonly" => Ok(BackupBindingAccess::RemoteReadonly),
        _ => bail!("backup bind access must be writable or remote-readonly"),
    }
}

#[cfg(test)]
mod tests {
    use super::bind_lines;

    const REPO_ID: &str = "11111111-1111-1111-1111-111111111111";

    #[test]
    fn plans_writable_binding_dry_run_without_persisting() {
        let lines = bind_lines(
            "s3://bucket-name/deve/",
            REPO_ID,
            "main",
            "writer-1",
            "writer-1",
            "writable",
            true,
        )
        .expect("bind dry-run");

        assert!(lines.iter().any(|line| line == "command=BindBackupTarget"));
        assert!(lines.iter().any(|line| line == "effect=BindingMutation"));
        assert!(lines.iter().any(|line| line == "dry_run=true"));
        assert!(
            lines
                .iter()
                .any(|line| line == "branch_path=deve/branches/writer-1")
        );
        assert!(lines.iter().any(|line| line == "access=Writable"));
        assert!(
            lines
                .iter()
                .any(|line| line == "writes_local_authority=false")
        );
    }

    #[test]
    fn rejects_non_local_writable_binding() {
        let err = bind_lines(
            "s3://bucket-name/deve/",
            REPO_ID,
            "main",
            "writer-2",
            "writer-1",
            "writable",
            true,
        )
        .expect_err("non-local writable binding must fail closed");

        assert!(err.to_string().contains("non-local backup writer"));
    }

    #[test]
    fn allows_non_local_remote_readonly_binding() {
        let lines = bind_lines(
            "s3://bucket-name/deve/",
            REPO_ID,
            "main",
            "writer-2",
            "writer-1",
            "remote-readonly",
            true,
        )
        .expect("remote readonly");

        assert!(lines.iter().any(|line| line == "access=RemoteReadonly"));
    }

    #[test]
    fn requires_dry_run_and_known_access() {
        let err = bind_lines(
            "s3://bucket-name/deve/",
            REPO_ID,
            "main",
            "writer-1",
            "writer-1",
            "writable",
            false,
        )
        .expect_err("dry-run required");
        assert!(err.to_string().contains("--dry-run"));

        let err = bind_lines(
            "s3://bucket-name/deve/",
            REPO_ID,
            "main",
            "writer-1",
            "writer-1",
            "write",
            true,
        )
        .expect_err("access rejected");
        assert!(err.to_string().contains("access must be"));
    }
}
