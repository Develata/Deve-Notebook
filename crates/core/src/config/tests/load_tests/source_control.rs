use super::super::{CWD_LOCK, CwdGuard, EnvGuard};
use crate::config::Config;

#[test]
fn source_control_git_bridge_config_file_is_rejected() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        "[source_control]\ngit_bridge = \"off\"\n",
    )
    .expect("config");

    let err = Config::load_checked().expect_err("git bridge config is unsupported");

    assert!(err.to_string().contains("Failed to parse configuration"));
}

#[test]
fn source_control_git_bridge_env_alias_is_rejected() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_SOURCE_CONTROL__GIT_BRIDGE", Some("off")),
        ("DEVE_SOURCE_CONTROL_GIT_BRIDGE", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("source control env alias unsupported");

    assert!(err.to_string().contains("Failed to parse configuration"));
}

#[test]
fn source_control_git_bridge_invalid_env_alias_is_rejected() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_SOURCE_CONTROL__GIT_BRIDGE", Some("native")),
        ("DEVE_SOURCE_CONTROL_GIT_BRIDGE", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("legacy git bridge env alias unsupported");

    assert!(err.to_string().contains("Failed to parse configuration"));
}

#[test]
fn source_control_ngit_only() {
    let default_output =
        toml::to_string_pretty(&Config::default()).expect("default config serializes");
    for legacy_key in ["git_bridge", "bridge_mode", "source_control.git_bridge"] {
        assert!(
            !default_output.contains(legacy_key),
            "default config must not expose legacy Source Control bridge key {legacy_key}"
        );
    }

    Config::from_toml_str_checked("[source_control]\n")
        .expect("empty source_control table remains supported");

    for legacy_config in [
        "[source_control]\ngit_bridge = \"off\"\n",
        "[source_control]\nmode = \"git\"\n",
        "[source_control]\nbridge_mode = \"off\"\n",
        "[source_control]\npolicy = \"git-authority\"\n",
    ] {
        let err =
            Config::from_toml_str_checked(legacy_config).expect_err("legacy SC mode must fail");
        assert!(
            err.to_string().contains("Failed to parse configuration"),
            "{err}"
        );
    }

    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_SOURCE_CONTROL__GIT_BRIDGE", Some("off")),
        ("DEVE_SOURCE_CONTROL__MODE", Some("git")),
        ("DEVE_SOURCE_CONTROL__BRIDGE_MODE", Some("off")),
        ("DEVE_SOURCE_CONTROL__POLICY", Some("git-authority")),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("legacy Source Control env keys unsupported");

    assert!(err.to_string().contains("Failed to parse configuration"));
}
