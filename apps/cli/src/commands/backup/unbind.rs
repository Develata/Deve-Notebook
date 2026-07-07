//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-command-output-contract
//!
//! Backup unbind planning and host-local binding removal.
//!
//! This command surface validates the target branch backup binding and prints
//! the planned unbind mutation. Without `--dry-run`, it removes secret-free
//! host-local binding metadata. It intentionally does not contact providers,
//! delete remote objects, write ledger state, persist credentials or keys, or
//! touch Projection Workspaces.

use anyhow::bail;
use deve_core::backup::{
    BackupBindingAccess, BackupBranchBindingInput, BackupCommandKind, BackupLocator,
    BackupPlanEffect, BackupPlanInput, backup_binding_status, backup_binding_store_path_for,
    backup_command_plan, list_backup_binding_records, plan_backup_branch_binding,
    remove_backup_branch_binding, validate_backup_branch_bindings,
};
use deve_core::models::RepoId;
use std::path::Path;

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

pub fn unbind(ledger_dir: &Path, input: UnbindBackupCommandInput<'_>) -> anyhow::Result<()> {
    for line in unbind_lines(ledger_dir, input)? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn unbind_lines(
    ledger_dir: &Path,
    input: UnbindBackupCommandInput<'_>,
) -> anyhow::Result<Vec<String>> {
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
    let existing = list_backup_binding_records(ledger_dir)?
        .into_iter()
        .find(|record| {
            record.locator == locator
                && record.binding.repo_id == binding.repo_id
                && record.binding.branch_name == binding.branch_name
                && record.binding.writer_identity == binding.writer_identity
                && record.binding.branch_path == binding.branch_path
        })
        .ok_or(deve_core::backup::BackupBindingStoreError::MissingBinding)?;
    if existing.binding.access != binding.access {
        bail!(
            "backup unbind access mismatch: existing={:?} requested={:?}",
            existing.binding.access,
            binding.access
        );
    }
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::UnbindBackupTarget,
        binding_status: backup_binding_status(Some(&existing.binding)),
        effect: BackupPlanEffect::BindingMutation,
    })?;
    if !input.dry_run {
        remove_backup_branch_binding(ledger_dir, &locator, &existing.binding)?;
    }

    let mut lines = vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("dry_run={}", input.dry_run),
        format!("binding_removed={}", !input.dry_run),
        format!("repo_id={}", binding.repo_id),
        format!("branch_name={}", binding.branch_name),
        format!("writer_identity={}", binding.writer_identity),
        format!("branch_path={}", binding.branch_path),
        format!("existing_access={:?}", existing.binding.access),
        format!("writes_local_authority={}", plan.writes_local_authority),
    ];
    if !input.dry_run {
        lines.push(format!(
            "binding_store={}",
            backup_binding_store_path_for(ledger_dir).display()
        ));
    }
    Ok(lines)
}

fn parse_access(access: &str) -> anyhow::Result<BackupBindingAccess> {
    match access {
        "writable" => Ok(BackupBindingAccess::Writable),
        "remote-readonly" => Ok(BackupBindingAccess::RemoteReadonly),
        _ => bail!("backup unbind access must be writable or remote-readonly"),
    }
}

#[cfg(test)]
mod tests;
