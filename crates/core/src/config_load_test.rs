use super::{CWD_LOCK, CwdGuard, EnvGuard};
use crate::config::{AppProfile, Config, MergeStrategy, SyncMode};

#[test]
fn test_default_config() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_AI__MODE", None),
        ("DEVE_AI__NATIVE_ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
        ("DEVE_AI_AGENT_BRIDGE_ENABLED", None),
        ("DEVE_AI_AGENT_BRIDGE_TRUSTED", None),
        ("AGENT_CLI_PATH", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load();

    assert!(!config.ledger_dir.is_empty());
    assert_eq!(config.ai.mode, "native");
    assert!(config.ai.native_enabled);
    assert!(!config.ai.agent_bridge.enabled);
    assert!(!config.ai.agent_bridge.trusted);
}

#[test]
fn default_config_matches_settings_plan_defaults() {
    let config = Config::default();

    assert_eq!(config.profile, AppProfile::Standard);
    assert_eq!(config.ledger_dir, "ledger");
    assert_eq!(config.vault_path, "vault");
    assert_eq!(config.sync_mode, SyncMode::Auto);
    assert_eq!(config.merge_strategy, MergeStrategy::Manual);
    assert_eq!(config.snapshot_depth, 100);
    assert_eq!(config.concurrency, 4);
    assert_eq!(config.ui.sidebar_width, 250);
    assert_eq!(config.ui.right_panel_width, 350);
    assert_eq!(config.ai.mode, "native");
    assert!(config.ai.native_enabled);
    assert!(!config.ai.agent_bridge.enabled);
    assert!(!config.ai.agent_bridge.trusted);
    assert_eq!(config.ai.agent_bridge.timeout_ms, 30_000);
}

#[test]
fn runtime_config_value_parsers_reject_unknown_values() {
    assert!("standard".parse::<AppProfile>().is_ok());
    assert!("low-spec".parse::<AppProfile>().is_ok());
    assert!("debug".parse::<AppProfile>().is_err());

    assert!("auto".parse::<SyncMode>().is_ok());
    assert!("manual".parse::<SyncMode>().is_ok());
    assert!("strict".parse::<SyncMode>().is_err());

    assert!("manual".parse::<MergeStrategy>().is_ok());
    assert!("auto".parse::<MergeStrategy>().is_ok());
    assert!("crdt".parse::<MergeStrategy>().is_err());
}

#[test]
fn load_checked_fails_closed_on_invalid_config_file() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(dir.path().join("config.toml"), "snapshot_depth = [").expect("bad config");

    let err = Config::load_checked().expect_err("invalid config must fail closed");

    assert!(
        err.to_string().contains("Failed to build configuration")
            || err.to_string().contains("Failed to parse configuration")
    );
}

#[test]
fn load_checked_fails_closed_on_invalid_runtime_enum_value() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(dir.path().join("config.toml"), "profile = \"debug\"\n").expect("bad profile");

    let err = Config::load_checked().expect_err("invalid profile must fail closed");

    assert!(err.to_string().contains("Failed to parse configuration"));
}

#[test]
fn load_checked_fails_closed_on_invalid_agent_bridge_alias_bool() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_AI_AGENT_BRIDGE_ENABLED", Some("maybe")),
        ("DEVE_AI_AGENT_BRIDGE_TRUSTED", None),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("invalid alias bool");

    assert!(
        err.to_string()
            .contains("Invalid boolean environment variable DEVE_AI_AGENT_BRIDGE_ENABLED")
    );
}

#[test]
fn env_overrides_flat_underscore_keys() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set(&[
        ("DEVE_LEDGER_DIR", "/tmp/deve-ledger"),
        ("DEVE_VAULT_PATH", "/tmp/deve-vault"),
        ("DEVE_SYNC_MODE", "manual"),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("env config");

    assert_eq!(config.ledger_dir, "/tmp/deve-ledger");
    assert_eq!(config.vault_path, "/tmp/deve-vault");
    assert_eq!(config.sync_mode, SyncMode::Manual);
}
