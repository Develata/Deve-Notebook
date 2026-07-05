use super::{Args, BackupAction, Commands};
use clap::Parser;

#[test]
fn backup_restore_accepts_flow_planning_args() {
    let args = Args::try_parse_from([
        "deve",
        "backup",
        "restore",
        "--locator",
        "s3://bucket-name/deve/",
        "--repo-id",
        "11111111-1111-1111-1111-111111111111",
        "--manifest-repo-id",
        "11111111-1111-1111-1111-111111111111",
        "--branch",
        "writer-1",
        "--manifest-digest",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "--pack-digest",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "--mode",
        "remote-readonly",
        "--manifest-verified",
        "--packs-downloaded",
        "--packs-decrypted",
        "--dry-run",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Backup {
            action:
                BackupAction::Restore {
                    locator,
                    repo_id,
                    manifest_repo_id,
                    branch,
                    manifest_digest,
                    pack_digests,
                    mode,
                    write_gate,
                    manifest_verified,
                    packs_downloaded,
                    packs_decrypted,
                    dry_run,
                },
        }) => {
            assert_eq!(locator, "s3://bucket-name/deve/");
            assert_eq!(repo_id, "11111111-1111-1111-1111-111111111111");
            assert_eq!(manifest_repo_id, "11111111-1111-1111-1111-111111111111");
            assert_eq!(branch, "writer-1");
            assert_eq!(
                manifest_digest,
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            );
            assert_eq!(
                pack_digests,
                vec!["bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"]
            );
            assert_eq!(mode, "remote-readonly");
            assert!(!write_gate);
            assert!(manifest_verified);
            assert!(packs_downloaded);
            assert!(packs_decrypted);
            assert!(dry_run);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}
