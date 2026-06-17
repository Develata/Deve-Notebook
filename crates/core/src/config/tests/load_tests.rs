use super::{CWD_LOCK, CwdGuard, EnvGuard};
use crate::config::{AppProfile, Config, GitBridgeMode, MergeStrategy, SyncMode};

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
        ("DEVE_PROFILE", None),
        ("DEVE_SNAPSHOT_DEPTH", None),
        ("DEVE_MEM_CACHE_MB", None),
        ("MEM_CACHE_MB", None),
        ("DEVE_SOURCE_CONTROL__GIT_BRIDGE", None),
        ("DEVE_SOURCE_CONTROL_GIT_BRIDGE", None),
        ("AGENT_CLI_PATH", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load();

    assert!(!config.ledger_dir.is_empty());
    assert_eq!(config.mem_cache_mb, 128);
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
    assert_eq!(config.sync_mode, SyncMode::Auto);
    assert_eq!(config.merge_strategy, MergeStrategy::Manual);
    assert_eq!(config.source_control.git_bridge, GitBridgeMode::Mirror);
    assert_eq!(config.snapshot_depth, 100);
    assert_eq!(config.mem_cache_mb, 128);
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
fn default_config_serializes_for_config_print() {
    let output =
        toml::to_string_pretty(&Config::default()).expect("default config serializes to toml");

    assert!(output.contains("snapshot_depth = 100"));
    assert!(output.contains("[source_control]"));
    assert!(output.contains("git_bridge = "));
    assert!(output.contains("[p2p]"));

    let config = toml::from_str::<Config>(&output).expect("printed config roundtrips");
    assert_eq!(config.source_control.git_bridge, GitBridgeMode::Mirror);
    assert_eq!(config.snapshot_depth, 100);
}

#[test]
fn from_toml_str_checked_runs_static_runtime_validation() {
    let err = Config::from_toml_str_checked(
        r#"
[p2p]
connect_interval_ms = 0
"#,
    )
    .expect_err("writer-side config validation must reject invalid runtime values");

    assert!(err.to_string().contains("p2p.connect_interval_ms"));
}

#[test]
fn source_control_git_bridge_config_file_accepts_mirror_and_off() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_SOURCE_CONTROL__GIT_BRIDGE", None),
        ("DEVE_SOURCE_CONTROL_GIT_BRIDGE", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        "[source_control]\ngit_bridge = \"off\"\n",
    )
    .expect("config");

    let config = Config::load_checked().expect("source control config");

    assert_eq!(config.source_control.git_bridge, GitBridgeMode::Off);
}

#[test]
fn source_control_git_bridge_env_alias_overrides_config_file() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_SOURCE_CONTROL__GIT_BRIDGE", Some("off")),
        ("DEVE_SOURCE_CONTROL_GIT_BRIDGE", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        "[source_control]\ngit_bridge = \"mirror\"\n",
    )
    .expect("config");

    let config = Config::load_checked().expect("source control env config");

    assert_eq!(config.source_control.git_bridge, GitBridgeMode::Off);
}

#[test]
fn source_control_git_bridge_fails_closed_on_invalid_value() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_SOURCE_CONTROL__GIT_BRIDGE", None),
        ("DEVE_SOURCE_CONTROL_GIT_BRIDGE", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        "[source_control]\ngit_bridge = \"native\"\n",
    )
    .expect("bad config");

    let err = Config::load_checked().expect_err("invalid git bridge mode");

    assert!(err.to_string().contains("Failed to parse configuration"));
}

#[test]
fn low_spec_profile_applies_unset_runtime_presets() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_PROFILE", Some("low-spec")),
        ("DEVE_SNAPSHOT_DEPTH", None),
        ("DEVE_MEM_CACHE_MB", None),
        ("MEM_CACHE_MB", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("low-spec config");

    assert_eq!(config.profile, AppProfile::LowSpec);
    assert_eq!(config.snapshot_depth, 10);
    assert_eq!(config.mem_cache_mb, 32);
}

#[test]
fn explicit_runtime_values_override_profile_presets() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_PROFILE", Some("low-spec")),
        ("DEVE_SNAPSHOT_DEPTH", Some("77")),
        ("DEVE_MEM_CACHE_MB", Some("88")),
        ("MEM_CACHE_MB", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("explicit runtime config");

    assert_eq!(config.profile, AppProfile::LowSpec);
    assert_eq!(config.snapshot_depth, 77);
    assert_eq!(config.mem_cache_mb, 88);
}

#[test]
fn mem_cache_mb_compat_env_alias_overrides_prefixed_config() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_PROFILE", Some("low-spec")),
        ("DEVE_MEM_CACHE_MB", Some("88")),
        ("MEM_CACHE_MB", Some("64")),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("mem cache alias config");

    assert_eq!(config.profile, AppProfile::LowSpec);
    assert_eq!(config.mem_cache_mb, 64);
}

#[test]
fn load_checked_fails_closed_on_invalid_mem_cache_env_alias() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_PROFILE", None),
        ("DEVE_MEM_CACHE_MB", None),
        ("MEM_CACHE_MB", Some("not-a-number")),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("invalid mem cache alias");

    assert!(
        err.to_string()
            .contains("Invalid integer environment variable MEM_CACHE_MB")
    );
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

    assert!("mirror".parse::<GitBridgeMode>().is_ok());
    assert!("off".parse::<GitBridgeMode>().is_ok());
    assert!("native".parse::<GitBridgeMode>().is_err());
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
    let _env = EnvGuard::set_optional(&[
        ("DEVE_LEDGER_DIR", Some("/tmp/deve-ledger")),
        ("DEVE_SYNC_MODE", Some("manual")),
        ("MEM_CACHE_MB", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("env config");

    assert_eq!(config.ledger_dir, "/tmp/deve-ledger");
    assert_eq!(config.sync_mode, SyncMode::Manual);
}

#[test]
fn load_checked_rejects_deve_vault_path_env() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[("DEVE_VAULT_PATH", Some("/tmp/deve-vault"))]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("DEVE_VAULT_PATH must fail closed");

    assert!(
        err.to_string()
            .contains("DEVE_VAULT_PATH is no longer supported")
    );
}

#[test]
fn load_checked_rejects_vault_path_config_key() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[("DEVE_VAULT_PATH", None)]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(dir.path().join("config.toml"), "vault_path = \"vault\"\n").expect("config");

    let err = Config::load_checked().expect_err("vault_path must fail closed");

    assert!(
        err.to_string()
            .contains("vault_path is no longer supported")
    );
}
