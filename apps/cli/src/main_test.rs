use super::{
    Args, Commands, ConfigAction, NgitAction, ProjectionRemoteAction, run_pre_config_command,
};
use crate::commands::projection_remote::ProjectionRemoteDirectionAction;
use clap::{CommandFactory, Parser};
use std::sync::Mutex;

mod backup;

static CWD_LOCK: Mutex<()> = Mutex::new(());

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
fn projection_remote_webdav_pull_accepts_locator() {
    let args = Args::try_parse_from([
        "deve",
        "projection-remote",
        "webdav",
        "pull",
        "--repo",
        "default",
        "--locator",
        "webdav+https://dav.example.com/notebooks/main",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::ProjectionRemote {
            action:
                ProjectionRemoteAction::Webdav {
                    action: ProjectionRemoteDirectionAction::Pull { repo, locator },
                },
        }) => {
            assert_eq!(repo.as_deref(), Some("default"));
            assert_eq!(locator, "webdav+https://dav.example.com/notebooks/main");
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
