use crate::{Args, BackupAction, Commands};
use clap::Parser;

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
