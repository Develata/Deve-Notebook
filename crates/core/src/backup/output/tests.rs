use super::*;
use crate::backup::{BackupBindingAccess, BackupBranchBinding};

fn binding(access: BackupBindingAccess) -> BackupBranchBinding {
    BackupBranchBinding {
        repo_id: uuid::Uuid::nil(),
        branch_name: "main".into(),
        writer_identity: "writer-1".into(),
        branch_path: "deve/branches/writer-1".into(),
        access,
    }
}

#[test]
fn derives_binding_status_from_branch_binding() {
    assert_eq!(backup_binding_status(None), BackupBindingStatus::Unbound);
    assert_eq!(
        backup_binding_status(Some(&binding(BackupBindingAccess::Writable))),
        BackupBindingStatus::Writable
    );
    assert_eq!(
        backup_binding_status(Some(&binding(BackupBindingAccess::RemoteReadonly))),
        BackupBindingStatus::RemoteReadonly
    );
}

#[test]
fn plans_backup_branch_only_for_writable_binding() {
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::BackupBranch,
        binding_status: BackupBindingStatus::Writable,
        effect: BackupPlanEffect::RemoteUpload,
    })
    .expect("writable backup plan");

    assert_eq!(plan.effect, BackupPlanEffect::RemoteUpload);
    assert!(!plan.writes_local_authority);

    let err = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::BackupBranch,
        binding_status: BackupBindingStatus::RemoteReadonly,
        effect: BackupPlanEffect::RemoteUpload,
    })
    .expect_err("readonly backup should fail closed");

    assert_eq!(err, BackupCommandOutputError::WritableBindingRequired);
}

#[test]
fn rejects_command_effect_mismatch() {
    let err = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::InspectBackupTarget,
        binding_status: BackupBindingStatus::Unbound,
        effect: BackupPlanEffect::RemoteUpload,
    })
    .expect_err("inspect must stay read-only");

    assert_eq!(err, BackupCommandOutputError::CommandEffectMismatch);
}

#[test]
fn blocks_conflicted_binding_before_effect_selection() {
    let err = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::BackupBranch,
        binding_status: BackupBindingStatus::Conflict,
        effect: BackupPlanEffect::RemoteUpload,
    })
    .expect_err("conflict must fail closed");

    assert_eq!(err, BackupCommandOutputError::BindingConflict);
}

#[test]
fn marks_explicit_restore_import_as_local_authority_write() {
    let plan = backup_command_plan(BackupPlanInput {
        command: BackupCommandKind::RestoreBackup,
        binding_status: BackupBindingStatus::RemoteReadonly,
        effect: BackupPlanEffect::ExplicitImport,
    })
    .expect("restore import plan");

    assert!(plan.writes_local_authority);
}

#[test]
fn builds_fail_closed_structured_error() {
    let error = BackupError::fail_closed(
        BackupCommandKind::VerifyBackupTarget,
        BackupErrorKind::PackHashMismatch,
    );

    assert_eq!(error.command, BackupCommandKind::VerifyBackupTarget);
    assert_eq!(error.kind, BackupErrorKind::PackHashMismatch);
    assert!(error.fail_closed);
    assert!(error.partial_effects_forbidden);
}
