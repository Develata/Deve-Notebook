use super::{Args, BackupAction, Commands};
use clap::Parser;

#[test]
fn backup_run_accepts_upload_plan_args() {
    let args = Args::try_parse_from([
        "deve",
        "backup",
        "run",
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
        "--credential-ref",
        "env:DEVE_BACKUP_TOKEN",
        "--key-ref",
        "keyring:deve/default-backup-key",
        "--pack-sequence",
        "1",
        "--ledger-start",
        "1",
        "--ledger-end",
        "1",
        "--ledger-events",
        "1",
        "--payload-digest",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "--encrypted",
        "--authenticated",
        "--dry-run",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Backup {
            action:
                BackupAction::Run {
                    locator,
                    repo_id,
                    branch_name,
                    writer,
                    local_writer,
                    credential_ref,
                    key_ref,
                    pack_sequence,
                    ledger_start,
                    ledger_end,
                    ledger_event_count,
                    payload_digest,
                    artifact,
                    encrypted,
                    authenticated,
                    dry_run,
                    ..
                },
        }) => {
            assert_eq!(locator, "s3://bucket-name/deve/");
            assert_eq!(repo_id, "11111111-1111-1111-1111-111111111111");
            assert_eq!(branch_name, "main");
            assert_eq!(writer, "writer-1");
            assert_eq!(local_writer, "writer-1");
            assert_eq!(credential_ref, "env:DEVE_BACKUP_TOKEN");
            assert_eq!(key_ref, "keyring:deve/default-backup-key");
            assert_eq!(pack_sequence, 1);
            assert_eq!(ledger_start, Some(1));
            assert_eq!(ledger_end, Some(1));
            assert_eq!(ledger_event_count, 1);
            assert_eq!(
                payload_digest,
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            );
            assert_eq!(artifact, None);
            assert!(encrypted);
            assert!(authenticated);
            assert!(dry_run);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}
