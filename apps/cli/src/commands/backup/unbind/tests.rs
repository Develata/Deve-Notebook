use super::{UnbindBackupCommandInput, unbind_lines};
use deve_core::backup::{
    BackupBindingAccess, BackupBranchBindingInput, BackupLocator, list_backup_binding_records,
    persist_backup_branch_binding, plan_backup_branch_binding,
};

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

fn persist_input_binding(
    ledger_dir: &std::path::Path,
    writer_identity: &str,
    access: BackupBindingAccess,
) {
    let locator = BackupLocator::parse("s3://bucket-name/deve/").expect("locator");
    let branch = locator.branch_locator(writer_identity).expect("branch");
    let repo_id = REPO_ID.parse().expect("repo id");
    let binding = plan_backup_branch_binding(BackupBranchBindingInput {
        repo_id,
        branch_name: "main".into(),
        writer_identity: branch.writer_identity,
        local_writer_identity: "writer-1".into(),
        branch_path: branch.branch_path,
        requested_access: access,
    })
    .expect("binding");
    persist_backup_branch_binding(ledger_dir, locator, binding).expect("persist");
}

#[test]
fn plans_unbind_without_removing_persisted_binding() {
    let dir = tempfile::tempdir().expect("tempdir");
    persist_input_binding(dir.path(), "writer-1", BackupBindingAccess::Writable);
    let lines = unbind_lines(dir.path(), input()).expect("unbind dry-run");

    assert!(
        lines
            .iter()
            .any(|line| line == "command=UnbindBackupTarget")
    );
    assert!(lines.iter().any(|line| line == "effect=BindingMutation"));
    assert!(lines.iter().any(|line| line == "binding_removed=false"));
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
    assert_eq!(
        list_backup_binding_records(dir.path())
            .expect("records")
            .len(),
        1
    );
}

#[test]
fn backup_unbind_removes_host_local_metadata_without_authority_writes() {
    let dir = tempfile::tempdir().expect("tempdir");
    persist_input_binding(dir.path(), "writer-1", BackupBindingAccess::Writable);
    let mut input = input();
    input.dry_run = false;

    let lines = unbind_lines(dir.path(), input).expect("unbind");

    assert!(lines.iter().any(|line| line == "dry_run=false"));
    assert!(lines.iter().any(|line| line == "binding_removed=true"));
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
fn supports_remote_readonly_unbind_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    persist_input_binding(dir.path(), "writer-2", BackupBindingAccess::RemoteReadonly);
    let mut input = input();
    input.writer_identity = "writer-2";
    input.access = "remote-readonly";
    let lines = unbind_lines(dir.path(), input).expect("remote readonly target");

    assert!(
        lines
            .iter()
            .any(|line| line == "existing_access=RemoteReadonly")
    );
}

#[test]
fn supports_non_dry_run_and_rejects_unknown_access() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut dry_run_input = input();
    dry_run_input.dry_run = false;
    let err = unbind_lines(dir.path(), dry_run_input).expect_err("missing binding");
    assert!(err.to_string().contains("does not exist"));

    let mut access_input = input();
    access_input.access = "write";
    let err = unbind_lines(dir.path(), access_input).expect_err("access rejected");
    assert!(err.to_string().contains("access must"));
}

#[test]
fn backup_unbind_dry_run_uses_persisted_access() {
    let dir = tempfile::tempdir().expect("tempdir");
    persist_input_binding(dir.path(), "writer-1", BackupBindingAccess::RemoteReadonly);
    let mut input = input();
    input.access = "writable";

    let err = unbind_lines(dir.path(), input).expect_err("access mismatch");

    assert!(err.to_string().contains("access mismatch"));
}
