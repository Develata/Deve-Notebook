use super::{Args, BackupAction, Commands};
use clap::Parser;

#[test]
fn backup_unbind_accepts_dry_run_binding_args() {
    let args = Args::try_parse_from([
        "deve",
        "backup",
        "unbind",
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
                BackupAction::Unbind {
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
