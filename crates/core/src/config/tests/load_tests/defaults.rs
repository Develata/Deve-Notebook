use super::super::{CWD_LOCK, CwdGuard, EnvGuard};
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
        ("DEVE_PROFILE", None),
        ("DEVE_SNAPSHOT_DEPTH", None),
        ("DEVE_MEM_CACHE_MB", None),
        ("MEM_CACHE_MB", None),
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
    assert!(!output.contains("git_bridge"));
    assert!(output.contains("[p2p]"));

    let config = toml::from_str::<Config>(&output).expect("printed config roundtrips");
    assert_eq!(config.snapshot_depth, 100);
}
