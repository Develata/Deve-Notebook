//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 18_backup#backup-branch-binding-contract
//!   - 18_backup#backup-command-output-contract
//!
//! Backup unbind dry-run planning.
//!
//! This command surface validates the target branch backup binding and prints
//! the planned unbind mutation. It intentionally does not persist binding
//! state, contact providers, delete remote objects, write ledger state, or
//! touch Projection Workspaces.

use anyhow::bail;
use deve_core::backup::{
    BackupBindingAccess, BackupBranchBindingInput, BackupCommandKind, BackupLocator,
    BackupPlanEffect, BackupPlanInput, backup_binding_status, backup_command_plan,
    plan_backup_branch_binding, validate_backup_branch_bindings,
};
use deve_core::models::RepoId;

#[derive(Clone, Copy)]
pub struct UnbindBackupCommandInput<'a> {
    pub locator: &'a str,
    pub repo_id: &'a str,
    pub branch_name: &'a str,
    pub writer_identity: &'a str,
    pub local_writer_identity: &'a str,
    pub access: &'a str,
    pub dry_run: bool,
}

pub fn unbind(input: UnbindBackupCommandInput<'_>) -> anyhow::Result<()> {
    for line in unbind_lines(input)? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn unbind_lines(input: UnbindBackupCommandInput<'_>) -> anyhow::Result<Vec<String>> {
    if !input.dry_run {
        bail!(
            "backup unbind currently requires --dry-run because binding persistence is not implemented"
        );
    }

    let locator = BackupLocator::parse(input.locator)?;
    let branch = locator.branch_locator(input.writer_identity)?;
    let repo_id: RepoId = input.repo_id.parse()?;
    let access = parse_access(input.access)?;
    let binding = plan_backup_branch_binding(BackupBranchBindingInput {
        repo_id,
        branch_name: input.branch_name.to_string(),
        writer_identity: branch.writer_identity,
        local_writer_identity: input.local_writer_identity.to_string(),
        branch_path: branch.branch_path,
        requested_access: access,
    })?;
    validate_backup_branch_bindings(std::slice::from_ref(&binding))?;
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::UnbindBackupTarget,
        binding_status: backup_binding_status(Some(&binding)),
        effect: BackupPlanEffect::BindingMutation,
    })?;

    Ok(vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("dry_run={}", input.dry_run),
        format!("repo_id={}", binding.repo_id),
        format!("branch_name={}", binding.branch_name),
        format!("writer_identity={}", binding.writer_identity),
        format!("branch_path={}", binding.branch_path),
        format!("existing_access={:?}", binding.access),
        format!("writes_local_authority={}", plan.writes_local_authority),
    ])
}

fn parse_access(access: &str) -> anyhow::Result<BackupBindingAccess> {
    match access {
        "writable" => Ok(BackupBindingAccess::Writable),
        "remote-readonly" => Ok(BackupBindingAccess::RemoteReadonly),
        _ => bail!("backup unbind access must be writable or remote-readonly"),
    }
}

#[cfg(test)]
mod tests {
    use super::{UnbindBackupCommandInput, unbind_lines};

    const REPO_ID: &str = "11111111-1111-1111-1111-111111111111";

    fn input() -> UnbindBackupCommandInput<'static> {
        UnbindBackupCommandInput {
            locator: "s3://bucket-name/deve/",
            repo_id: REPO_ID,
            branch_name: "main",
            writer_identity: "writer-1",
            local_writer_identity: "writer-1",
            access: "writable",
            dry_run: true,
        }
    }

    #[test]
    fn plans_unbind_without_persisting() {
        let lines = unbind_lines(input()).expect("unbind dry-run");

        assert!(
            lines
                .iter()
                .any(|line| line == "command=UnbindBackupTarget")
        );
        assert!(lines.iter().any(|line| line == "effect=BindingMutation"));
        assert!(lines.iter().any(|line| line == "existing_access=Writable"));
        assert!(
            lines
                .iter()
                .any(|line| line == "branch_path=deve/branches/writer-1")
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "writes_local_authority=false")
        );
    }

    #[test]
    fn supports_remote_readonly_unbind_target() {
        let mut input = input();
        input.writer_identity = "writer-2";
        input.access = "remote-readonly";
        let lines = unbind_lines(input).expect("remote readonly target");

        assert!(
            lines
                .iter()
                .any(|line| line == "existing_access=RemoteReadonly")
        );
    }

    #[test]
    fn requires_dry_run_and_known_access() {
        let mut dry_run_input = input();
        dry_run_input.dry_run = false;
        let err = unbind_lines(dry_run_input).expect_err("dry-run required");
        assert!(err.to_string().contains("--dry-run"));

        let mut access_input = input();
        access_input.access = "write";
        let err = unbind_lines(access_input).expect_err("access rejected");
        assert!(err.to_string().contains("access must"));
    }
}
