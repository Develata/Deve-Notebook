use super::{Args, Commands, ConfigAction, run_pre_config_command};
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
