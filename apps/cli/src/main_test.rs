use super::{Args, Commands, ConfigAction, GitAction, run_pre_config_command};
use clap::Parser;
use std::sync::Mutex;

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
fn git_status_accepts_repo_selector() {
    let args =
        Args::try_parse_from(["deve", "git", "status", "--repo", "default"]).expect("parse args");

    match args.command {
        Some(Commands::Git {
            action: GitAction::Status { repo },
        }) => assert_eq!(repo.as_deref(), Some("default")),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn git_mirror_accepts_retry_out_of_sync() {
    let args = Args::try_parse_from([
        "deve",
        "git",
        "mirror",
        "--repo",
        "default",
        "--retry-out-of-sync",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Git {
            action:
                GitAction::Mirror {
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
fn git_export_accepts_retry_out_of_sync() {
    let args = Args::try_parse_from([
        "deve",
        "git",
        "export",
        "--repo",
        "default",
        "--retry-out-of-sync",
    ])
    .expect("parse args");

    match args.command {
        Some(Commands::Git {
            action:
                GitAction::Export {
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
fn git_import_accepts_repo_selector() {
    let args = Args::try_parse_from(["deve", "git", "import", "--repo", "default", "--apply"])
        .expect("parse args");

    match args.command {
        Some(Commands::Git {
            action: GitAction::Import { repo, apply },
        }) => {
            assert_eq!(repo.as_deref(), Some("default"));
            assert!(apply);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}
