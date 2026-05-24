use crate::{Args, BackupAction, Commands};
use clap::Parser;

mod restore;

#[test]
fn backup_bind_accepts_dry_run_binding_args() {
    let args = Args::try_parse_from([
        "deve",
        "backup",
        "bind",
        "--locator",
        "s3://bucket-name/deve/",
        "--repo-id",
        "11111111-1111-1111-1111-111111111111",
        "--branch-name",
        "main",
        "--writer",
        "writer-1",
        "--local-writer",
        "writer-1",
        "--access",
        "writable",
        "--dry-run",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Backup {
            action:
                BackupAction::Bind {
                    locator,
                    repo_id,
                    branch_name,
                    writer,
                    local_writer,
                    access,
                    dry_run,
                },
        }) => {
            assert_eq!(locator, "s3://bucket-name/deve/");
            assert_eq!(repo_id, "11111111-1111-1111-1111-111111111111");
            assert_eq!(branch_name, "main");
            assert_eq!(writer, "writer-1");
            assert_eq!(local_writer, "writer-1");
            assert_eq!(access, "writable");
            assert!(dry_run);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn backup_inspect_accepts_locator_and_branch() {
    let args = Args::try_parse_from([
        "deve",
        "backup",
        "inspect",
        "--locator",
        "s3://bucket-name/deve/",
        "--branch",
        "writer-1",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Backup {
            action:
                BackupAction::Inspect {
                    locator,
                    branch,
                    credential_ref,
                    key_ref,
                },
        }) => {
            assert_eq!(locator, "s3://bucket-name/deve/");
            assert_eq!(branch.as_deref(), Some("writer-1"));
            assert_eq!(credential_ref, None);
            assert_eq!(key_ref, None);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn backup_inspect_accepts_credential_and_key_refs() {
    let args = Args::try_parse_from([
        "deve",
        "backup",
        "inspect",
        "--locator",
        "webdav+https://dav.example.com/deve/",
        "--credential-ref",
        "env:DEVE_BACKUP_TOKEN",
        "--key-ref",
        "keyring:deve/default-backup-key",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Backup {
            action:
                BackupAction::Inspect {
                    credential_ref,
                    key_ref,
                    ..
                },
        }) => {
            assert_eq!(credential_ref.as_deref(), Some("env:DEVE_BACKUP_TOKEN"));
            assert_eq!(key_ref.as_deref(), Some("keyring:deve/default-backup-key"));
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn backup_list_accepts_locator_and_object_paths() {
    let args = Args::try_parse_from([
        "deve",
        "backup",
        "list",
        "--locator",
        "s3://bucket-name/deve/",
        "--object",
        "deve/repo.manifest.enc",
        "--object",
        "deve/branches/writer-1/branch.manifest.enc",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Backup {
            action: BackupAction::List { locator, objects },
        }) => {
            assert_eq!(locator, "s3://bucket-name/deve/");
            assert_eq!(
                objects,
                vec![
                    "deve/repo.manifest.enc".to_string(),
                    "deve/branches/writer-1/branch.manifest.enc".to_string()
                ]
            );
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn backup_verify_accepts_locator_branch_objects_and_expected_packs() {
    let args = Args::try_parse_from([
        "deve",
        "backup",
        "verify",
        "--locator",
        "s3://bucket-name/deve/",
        "--branch",
        "writer-1",
        "--object",
        "deve/repo.manifest.enc",
        "--object",
        "deve/branches/writer-1/branch.manifest.enc",
        "--pack",
        "deve/branches/writer-1/packs/000001.pack.enc",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Backup {
            action:
                BackupAction::Verify {
                    locator,
                    branch,
                    objects,
                    expected_packs,
                },
        }) => {
            assert_eq!(locator, "s3://bucket-name/deve/");
            assert_eq!(branch, "writer-1");
            assert_eq!(
                objects,
                vec![
                    "deve/repo.manifest.enc".to_string(),
                    "deve/branches/writer-1/branch.manifest.enc".to_string()
                ]
            );
            assert_eq!(
                expected_packs,
                vec!["deve/branches/writer-1/packs/000001.pack.enc".to_string()]
            );
        }
        other => panic!("unexpected command: {other:?}"),
    }
}
