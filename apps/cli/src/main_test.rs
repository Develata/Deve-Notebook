use super::{
    Args, Commands, ConfigAction, NgitAction, ProjectionRemoteAction, RepoAction, RepoAliasAction,
    run_pre_config_command,
};
use crate::commands::projection_remote::{S3ProjectionProfileAction, S3ProjectionRemoteAction};
use clap::{CommandFactory, Parser};
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn repo_alias_commands_expose_set_export_and_dry_run_import() {
    let repo_id = uuid::Uuid::new_v4();
    let set = Args::try_parse_from([
        "deve",
        "repo",
        "alias",
        "set",
        "--repo-id",
        &repo_id.to_string(),
        "--alias",
        "math",
        "--expected-revision",
        "3",
    ])
    .expect("parse alias set");
    assert!(matches!(
        set.command,
        Some(Commands::Repo {
            action: RepoAction::Alias {
                action: RepoAliasAction::Set {
                    repo_id: parsed,
                    expected_revision: 3,
                    ..
                }
            }
        }) if parsed == repo_id
    ));

    let export = Args::try_parse_from([
        "deve",
        "repo",
        "alias",
        "export",
        "--output",
        "aliases.json",
    ])
    .expect("parse alias export");
    assert!(matches!(
        export.command,
        Some(Commands::Repo {
            action: RepoAction::Alias {
                action: RepoAliasAction::Export { .. }
            }
        })
    ));

    let import =
        Args::try_parse_from(["deve", "repo", "alias", "import", "--input", "aliases.json"])
            .expect("parse alias import");
    assert!(matches!(
        import.command,
        Some(Commands::Repo {
            action: RepoAction::Alias {
                action: RepoAliasAction::Import { apply: false, .. }
            }
        })
    ));
}

#[test]
fn export_accepts_out_alias_for_output() {
    let args = Args::try_parse_from([
        "deve",
        "export",
        "--format",
        "markdown",
        "--doc",
        "123e4567-e89b-12d3-a456-426614174000",
        "--out",
        "/tmp/export.md",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Export {
            output,
            doc,
            format,
            ..
        }) => {
            assert_eq!(output.as_deref(), Some("/tmp/export.md"));
            assert_eq!(doc.as_deref(), Some("123e4567-e89b-12d3-a456-426614174000"));
            assert_eq!(format, "markdown");
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn graph_accepts_repo_and_out_alias_for_projection_output() {
    let args = Args::try_parse_from([
        "deve",
        "graph",
        "--repo",
        "default",
        "--out",
        "/tmp/graph.json",
        "--pretty",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Graph {
            repo,
            output,
            pretty,
            allow_degraded_projection,
        }) => {
            assert_eq!(repo.as_deref(), Some("default"));
            assert_eq!(output.as_deref(), Some("/tmp/graph.json"));
            assert!(pretty);
            assert!(!allow_degraded_projection);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn sc_status_accepts_repo_selector() {
    let args =
        Args::try_parse_from(["deve", "sc-status", "--repo", "default"]).expect("parse args");

    match args.command {
        Some(Commands::ScStatus { repo }) => {
            assert_eq!(repo.as_deref(), Some("default"));
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn repair_defaults_to_projection_workspace_backup_root() {
    let args = Args::try_parse_from(["deve", "repair", "--check"]).expect("parse args");

    match args.command {
        Some(Commands::Repair { backup, .. }) => {
            assert_eq!(
                backup,
                std::path::PathBuf::from("backups/projection-workspace")
            );
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn config_set_runs_before_loading_runtime_config() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempfile::tempdir().expect("tempdir");
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");
    std::fs::write(dir.path().join("config.toml"), "profile = \"broken\"\n")
        .expect("seed invalid runtime config");
    let command = Some(Commands::Config {
        action: ConfigAction::Set {
            key: "profile".into(),
            value: "standard".into(),
        },
    });

    assert!(run_pre_config_command(&command).expect("run pre-config command"));
    let loaded = deve_core::config::Config::load_checked().expect("repaired config loads");

    std::env::set_current_dir(old_cwd).expect("restore cwd");
    assert_eq!(loaded.profile, deve_core::config::AppProfile::Standard);
}

#[test]
fn ngit_status_accepts_repo_selector() {
    let args =
        Args::try_parse_from(["deve", "ngit", "status", "--repo", "default"]).expect("parse args");

    match args.command {
        Some(Commands::Ngit {
            action: NgitAction::Status { repo },
        }) => assert_eq!(repo.as_deref(), Some("default")),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn ngit_mirror_accepts_retry_out_of_sync() {
    let args = Args::try_parse_from([
        "deve",
        "ngit",
        "mirror",
        "--repo",
        "default",
        "--retry-out-of-sync",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Ngit {
            action:
                NgitAction::Mirror {
                    repo,
                    retry_out_of_sync,
                },
        }) => {
            assert_eq!(repo.as_deref(), Some("default"));
            assert!(retry_out_of_sync);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn ngit_export_accepts_retry_out_of_sync() {
    let args = Args::try_parse_from([
        "deve",
        "ngit",
        "export",
        "--repo",
        "default",
        "--retry-out-of-sync",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Ngit {
            action:
                NgitAction::Export {
                    repo,
                    retry_out_of_sync,
                },
        }) => {
            assert_eq!(repo.as_deref(), Some("default"));
            assert!(retry_out_of_sync);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn ngit_import_accepts_repo_selector() {
    let args = Args::try_parse_from(["deve", "ngit", "import", "--repo", "default", "--apply"])
        .expect("parse args");

    match args.command {
        Some(Commands::Ngit {
            action: NgitAction::Import { repo, apply },
        }) => {
            assert_eq!(repo.as_deref(), Some("default"));
            assert!(apply);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn ngit_push_accepts_repo_remote_and_branch() {
    let args = Args::try_parse_from([
        "deve", "ngit", "push", "--repo", "default", "--remote", "origin", "--branch", "main",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Ngit {
            action:
                NgitAction::Push {
                    repo,
                    remote,
                    branch,
                },
        }) => {
            assert_eq!(repo.as_deref(), Some("default"));
            assert_eq!(remote.as_deref(), Some("origin"));
            assert_eq!(branch.as_deref(), Some("main"));
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn projection_remote_webdav_pull_is_rejected() {
    let error = Args::try_parse_from([
        "deve",
        "projection-remote",
        "webdav",
        "pull",
        "--repo",
        "default",
        "--locator",
        "webdav+https://dav.example.com/notebooks/main",
    ])
    .expect_err("unpublished pull command must be removed");

    assert!(error.to_string().contains("unrecognized subcommand 'pull'"));
}

#[test]
fn projection_remote_s3_push_accepts_explicit_profile() {
    let args = Args::try_parse_from([
        "deve",
        "projection-remote",
        "s3",
        "push",
        "--repo",
        "default",
        "--locator",
        "s3+https://minio.example.com/bucket/notebooks/main",
        "--profile",
        "minio",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::ProjectionRemote {
            action:
                ProjectionRemoteAction::S3 {
                    action:
                        S3ProjectionRemoteAction::Push {
                            repo,
                            locator,
                            profile,
                        },
                },
        }) => {
            assert_eq!(repo.as_deref(), Some("default"));
            assert_eq!(
                locator,
                "s3+https://minio.example.com/bucket/notebooks/main"
            );
            assert_eq!(profile.as_deref(), Some("minio"));
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn projection_remote_s3_profile_put_accepts_secret_free_fields() {
    let args = Args::try_parse_from([
        "deve",
        "projection-remote",
        "s3",
        "profile",
        "put",
        "--profile",
        "minio",
        "--endpoint-origin",
        "https://minio.example.com",
        "--bucket",
        "bucket",
        "--allowed-prefix",
        "notebooks/main",
        "--region",
        "us-east-1",
        "--credential-env-prefix",
        "MINIO",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::ProjectionRemote {
            action:
                ProjectionRemoteAction::S3 {
                    action:
                        S3ProjectionRemoteAction::Profile {
                            action:
                                S3ProjectionProfileAction::Put {
                                    profile,
                                    endpoint_origin,
                                    bucket,
                                    allowed_prefix,
                                    region,
                                    credential_env_prefix,
                                    allowed_capabilities,
                                },
                        },
                },
        }) => {
            assert_eq!(profile, "minio");
            assert_eq!(endpoint_origin, "https://minio.example.com");
            assert_eq!(bucket, "bucket");
            assert_eq!(allowed_prefix, "notebooks/main");
            assert_eq!(region, "us-east-1");
            assert_eq!(credential_env_prefix, "MINIO");
            assert_eq!(allowed_capabilities, vec!["push", "source-acquisition"]);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn top_level_help_uses_projection_workspace_language() {
    let help = Args::command().render_long_help().to_string();

    assert!(help.contains("Projection Locator"));
    assert!(help.contains("projection workspaces"));
    assert!(!help.contains("vault"));
    assert!(!help.contains("Vault_old"));
}
