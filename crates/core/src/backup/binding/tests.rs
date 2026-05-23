use super::{
    BackupBindingAccess, BackupBindingError, BackupBranchBinding, BackupBranchBindingInput,
    plan_backup_branch_binding, validate_backup_branch_bindings,
};

fn repo_id() -> uuid::Uuid {
    uuid::Uuid::from_u128(13)
}

fn input() -> BackupBranchBindingInput {
    BackupBranchBindingInput {
        repo_id: repo_id(),
        branch_name: "main".into(),
        writer_identity: "writer-1".into(),
        local_writer_identity: "writer-1".into(),
        branch_path: "deve/branches/writer-1/".into(),
        requested_access: BackupBindingAccess::Writable,
    }
}

fn binding(
    branch_name: &str,
    writer_identity: &str,
    branch_path: &str,
    access: BackupBindingAccess,
) -> BackupBranchBinding {
    BackupBranchBinding {
        repo_id: repo_id(),
        branch_name: branch_name.into(),
        writer_identity: writer_identity.into(),
        branch_path: branch_path.into(),
        access,
    }
}

#[test]
fn plans_local_writable_binding_with_normalized_path() {
    let binding = plan_backup_branch_binding(input()).unwrap();

    assert_eq!(binding.branch_name, "main");
    assert_eq!(binding.writer_identity, "writer-1");
    assert_eq!(binding.branch_path, "deve/branches/writer-1");
    assert_eq!(binding.access, BackupBindingAccess::Writable);
}

#[test]
fn non_local_writer_must_be_remote_readonly() {
    let mut binding_input = input();
    binding_input.writer_identity = "writer-2".into();
    assert!(matches!(
        plan_backup_branch_binding(binding_input),
        Err(BackupBindingError::NonLocalWriterMustBeReadonly)
    ));

    let mut binding_input = input();
    binding_input.writer_identity = "writer-2".into();
    binding_input.requested_access = BackupBindingAccess::RemoteReadonly;
    let binding = plan_backup_branch_binding(binding_input).unwrap();
    assert_eq!(binding.access, BackupBindingAccess::RemoteReadonly);
}

#[test]
fn duplicate_writable_branch_fails_closed() {
    let bindings = vec![
        binding(
            "main",
            "writer-1",
            "deve/branches/writer-1",
            BackupBindingAccess::Writable,
        ),
        binding(
            "main",
            "writer-2",
            "deve/branches/writer-2",
            BackupBindingAccess::Writable,
        ),
    ];

    assert!(matches!(
        validate_backup_branch_bindings(&bindings),
        Err(BackupBindingError::DuplicateWritableBranch)
    ));
}

#[test]
fn duplicate_active_writer_path_fails_closed() {
    let bindings = vec![
        binding(
            "main",
            "writer-1",
            "deve/branches/shared",
            BackupBindingAccess::Writable,
        ),
        binding(
            "feature",
            "writer-2",
            "deve/branches/shared",
            BackupBindingAccess::Writable,
        ),
    ];

    assert!(matches!(
        validate_backup_branch_bindings(&bindings),
        Err(BackupBindingError::DuplicateActiveWriterPath)
    ));
}

#[test]
fn remote_readonly_binding_does_not_create_active_writer_conflict() {
    let bindings = vec![
        binding(
            "main",
            "writer-1",
            "deve/branches/writer-1",
            BackupBindingAccess::Writable,
        ),
        binding(
            "feature",
            "writer-2",
            "deve/branches/writer-1",
            BackupBindingAccess::RemoteReadonly,
        ),
    ];

    validate_backup_branch_bindings(&bindings).unwrap();
}

#[test]
fn rejects_duplicate_branch_writer_and_unsafe_keys() {
    let bindings = vec![
        binding(
            "main",
            "writer-1",
            "deve/branches/writer-1",
            BackupBindingAccess::RemoteReadonly,
        ),
        binding(
            "main",
            "writer-1",
            "deve/branches/writer-1-copy",
            BackupBindingAccess::RemoteReadonly,
        ),
    ];
    assert!(matches!(
        validate_backup_branch_bindings(&bindings),
        Err(BackupBindingError::DuplicateBranchWriterBinding)
    ));

    let mut binding_input = input();
    binding_input.branch_name = "bad/name".into();
    assert!(matches!(
        plan_backup_branch_binding(binding_input),
        Err(BackupBindingError::UnsafeBranchName(_))
    ));

    let mut binding_input = input();
    binding_input.branch_path = "deve//branches/writer-1".into();
    assert!(plan_backup_branch_binding(binding_input).is_err());
}
