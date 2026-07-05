//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-command-output-contract
//!
//! Backup bind command planning and host-local binding persistence.
//!
//! This command surface validates branch backup binding inputs and prints the
//! planned binding. Without `--dry-run`, it persists secret-free host-local
//! binding metadata. It intentionally does not contact providers, write ledger
//! state, persist credentials or keys, or touch Projection Workspaces.

use anyhow::bail;
use deve_core::backup::{
    BackupBindingAccess, BackupBindingStatus, BackupBranchBindingInput, BackupCommandKind,
    BackupLocator, BackupPlanEffect, BackupPlanInput, backup_binding_store_path_for,
    backup_command_plan, persist_backup_branch_binding, plan_backup_branch_binding,
    validate_backup_branch_bindings,
};
use deve_core::models::RepoId;
use std::path::Path;

#[derive(Clone, Copy)]
pub struct BindBackupCommandInput<'a> {
    pub locator: &'a str,
    pub repo_id: &'a str,
    pub branch_name: &'a str,
    pub writer_identity: &'a str,
    pub local_writer_identity: &'a str,
    pub access: &'a str,
    pub dry_run: bool,
}

pub fn bind(ledger_dir: &Path, input: BindBackupCommandInput<'_>) -> anyhow::Result<()> {
    for line in bind_lines(ledger_dir, input)? {
        println!("{line}");
    }
    Ok(())
}

pub(crate) fn bind_lines(
    ledger_dir: &Path,
    input: BindBackupCommandInput<'_>,
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
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::BindBackupTarget,
        binding_status: BackupBindingStatus::Unbound,
        effect: BackupPlanEffect::BindingMutation,
    })?;
    if !input.dry_run {
        persist_backup_branch_binding(ledger_dir, locator.clone(), binding.clone())?;
    }

    let mut lines = vec![
        format!("backup_locator: provider={}", locator.provider.protocol()),
        format!("command={:?}", plan.command),
        format!("effect={:?}", plan.effect),
        format!("dry_run={}", input.dry_run),
        format!("binding_persisted={}", !input.dry_run),
        format!("repo_id={}", binding.repo_id),
        format!("branch_name={}", binding.branch_name),
        format!("writer_identity={}", binding.writer_identity),
        format!("branch_path={}", binding.branch_path),
        format!("access={:?}", binding.access),
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
        _ => bail!("backup bind access must be writable or remote-readonly"),
    }
}

#[cfg(test)]
mod tests {
    use super::{BindBackupCommandInput, bind_lines};
    use deve_core::backup::list_backup_binding_records;

    const REPO_ID: &str = "11111111-1111-1111-1111-111111111111";

    fn input() -> BindBackupCommandInput<'static> {
        BindBackupCommandInput {
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
    fn plans_writable_binding_dry_run_without_persisting() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lines = bind_lines(dir.path(), input()).expect("bind dry-run");

        assert!(lines.iter().any(|line| line == "command=BindBackupTarget"));
        assert!(lines.iter().any(|line| line == "effect=BindingMutation"));
        assert!(lines.iter().any(|line| line == "dry_run=true"));
        assert!(lines.iter().any(|line| line == "binding_persisted=false"));
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
        assert!(
            list_backup_binding_records(dir.path())
                .expect("records")
                .is_empty()
        );
    }

    #[test]
    fn backup_bind_persists_host_local_metadata_without_authority_writes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut input = input();
        input.dry_run = false;

        let lines = bind_lines(dir.path(), input).expect("bind");

        assert!(lines.iter().any(|line| line == "dry_run=false"));
        assert!(lines.iter().any(|line| line == "binding_persisted=true"));
        assert!(
            lines
                .iter()
                .any(|line| line == "writes_local_authority=false")
        );
        let records = list_backup_binding_records(dir.path()).expect("records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].binding.branch_name, "main");
        assert_eq!(records[0].binding.writer_identity, "writer-1");
    }

    #[test]
    fn rejects_non_local_writable_binding() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut input = input();
        input.writer_identity = "writer-2";

        let err =
            bind_lines(dir.path(), input).expect_err("non-local writable binding must fail closed");

        assert!(err.to_string().contains("non-local backup writer"));
    }

    #[test]
    fn allows_non_local_remote_readonly_binding() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut input = input();
        input.writer_identity = "writer-2";
        input.access = "remote-readonly";

        let lines = bind_lines(dir.path(), input).expect("remote readonly");

        assert!(lines.iter().any(|line| line == "access=RemoteReadonly"));
    }

    #[test]
    fn supports_non_dry_run_and_rejects_unknown_access() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut bind_input = input();
        bind_input.dry_run = false;

        let lines = bind_lines(dir.path(), bind_input).expect("non-dry-run bind is supported");
        assert!(lines.iter().any(|line| line == "binding_persisted=true"));

        let mut access_input = input();
        access_input.access = "write";
        access_input.dry_run = true;
        let err = bind_lines(dir.path(), access_input).expect_err("access rejected");
        assert!(err.to_string().contains("access must be"));
    }
}
