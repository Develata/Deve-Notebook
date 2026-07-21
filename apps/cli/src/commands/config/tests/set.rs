use super::super::set_in_file;
use deve_core::config::{AppProfile, Config};

#[test]
fn set_core_key_writes_runtime_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    set_in_file(&path, "profile", "low-spec").expect("set profile");

    let output = std::fs::read_to_string(path).expect("read config");
    let config: Config = toml::from_str(&output).expect("valid config");
    assert_eq!(config.profile, AppProfile::LowSpec);
}

#[test]
fn set_ui_key_is_preserved_without_breaking_runtime_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    set_in_file(&path, "ui.sidebar_width", "300").expect("set ui");

    let output = std::fs::read_to_string(path).expect("read config");
    let config = toml::from_str::<Config>(&output).expect("runtime-compatible config");
    assert_eq!(config.ui.sidebar_width, 300);
    assert!(output.contains("sidebar_width = 300"));
}

#[test]
fn set_repo_creation_projection_base_round_trips_through_runtime_validation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let projection_base = dir.path().join("notes");

    set_in_file(
        &path,
        "repo_creation_projection_base",
        &projection_base.to_string_lossy(),
    )
    .expect("set absolute projection base");

    let output = std::fs::read_to_string(path).expect("read config");
    let config = Config::from_toml_str_checked(&output).expect("runtime-valid config");
    assert_eq!(
        config.repo_creation_projection_base.as_deref(),
        Some(projection_base.as_path())
    );
}

#[test]
fn set_rejects_relative_repo_creation_projection_base_without_rewriting() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "profile = \"standard\"\n";
    std::fs::write(&path, original).expect("seed config");

    let error = set_in_file(&path, "repo_creation_projection_base", "relative/notes")
        .expect_err("relative base must fail closed");

    assert!(error.to_string().contains("Invalid absolute path"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_unknown_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "profile = \"standard\"\n";
    std::fs::write(&path, original).expect("seed config");

    let err = set_in_file(&path, "unknown.key", "1").expect_err("reject key");
    assert!(err.to_string().contains("Unsupported config key"));

    let err = set_in_file(&path, "server.settings.api_enabled", "true").expect_err("reject future");
    assert!(err.to_string().contains("Unsupported config key"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_invalid_value_without_rewriting_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "profile = \"standard\"\n";
    std::fs::write(&path, original).expect("seed config");

    let invalid_choice = set_in_file(&path, "profile", "invalid").expect_err("reject choice");
    assert!(invalid_choice.to_string().contains("Invalid value"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );

    let invalid_integer = set_in_file(&path, "ui.sidebar_width", "-1").expect_err("reject integer");
    assert!(
        invalid_integer
            .to_string()
            .contains("Integer config values must be non-negative")
    );
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_empty_env_reference_without_rewriting_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "[p2p]\ninbound_token_env = \"DEVE_P2P_INBOUND_TOKEN\"\n";
    std::fs::write(&path, original).expect("seed config");

    let err = set_in_file(&path, "p2p.inbound_token_env", "\"\"")
        .expect_err("empty env reference must fail closed");

    assert!(
        err.to_string()
            .contains("non-empty environment variable name")
    );
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_zero_p2p_connect_interval_without_rewriting_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "[p2p]\nconnect_interval_ms = 5000\n";
    std::fs::write(&path, original).expect("seed config");

    let err = set_in_file(&path, "p2p.connect_interval_ms", "0")
        .expect_err("zero p2p connect interval must fail closed");

    assert!(err.to_string().contains(">= 1"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_existing_invalid_runtime_config_without_rewriting() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = r#"[[p2p.peers]]
label = "peer-b"
peer_id = "peer-b"
repo_id = "11111111-1111-1111-1111-111111111111"
ws_url = "ws://peer-b:3001/ws"
auth_token_env = "DEVE_P2P_PEER_B_TOKEN"
"#;
    std::fs::write(&path, original).expect("seed config");

    let err = set_in_file(&path, "ui.sidebar_width", "300")
        .expect_err("existing invalid runtime config must fail closed");

    assert!(err.to_string().contains("p2p.peers[0].peer_id"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_invalid_env_reference_name_without_rewriting_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "[p2p]\ninbound_token_env = \"DEVE_P2P_INBOUND_TOKEN\"\n";
    std::fs::write(&path, original).expect("seed config");

    let err = set_in_file(&path, "p2p.inbound_token_env", "1BAD")
        .expect_err("invalid env name must fail closed");

    assert!(err.to_string().contains("valid environment variable name"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_nested_key_when_parent_is_scalar() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "ui = \"scalar\"\n";
    std::fs::write(&path, original).expect("seed config");

    let err = set_in_file(&path, "ui.sidebar_width", "300").expect_err("reject scalar parent");
    assert!(err.to_string().contains("ui is already a scalar"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}
