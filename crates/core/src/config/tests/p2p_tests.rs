use super::{CWD_LOCK, CwdGuard, EnvGuard};
use crate::config::Config;

#[test]
fn p2p_mesh_env_aliases_load_static_peer_config() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_P2P__ENABLED", Some("true")),
        ("DEVE_P2P__CONNECT_INTERVAL_MS", Some("1234")),
        (
            "DEVE_P2P__INBOUND_TOKEN_ENV",
            Some("DEVE_P2P_INBOUND_TOKEN"),
        ),
        ("DEVE_P2P_MESH_PEER_0_LABEL", Some("peer-b")),
        ("DEVE_P2P_MESH_PEER_0_PEER_ID", Some("0123456789ab")),
        (
            "DEVE_P2P_MESH_PEER_0_REPO_ID",
            Some("11111111-1111-1111-1111-111111111111"),
        ),
        ("DEVE_P2P_MESH_PEER_0_WS_URL", Some("ws://peer-b:3001/ws")),
        (
            "DEVE_P2P_MESH_PEER_0_AUTH_TOKEN_ENV",
            Some("DEVE_P2P_PEER_B_TOKEN"),
        ),
        ("DEVE_P2P_MESH_PEER_0_ENABLED", Some("true")),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("p2p env config");

    assert!(config.p2p.enabled);
    assert_eq!(config.p2p.connect_interval_ms, 1234);
    assert_eq!(config.p2p.peers.len(), 1);
    assert_eq!(config.p2p.peers[0].label, "peer-b");
    assert_eq!(config.p2p.peers[0].peer_id, "0123456789ab");
    assert_eq!(config.p2p.peers[0].auth_token_env, "DEVE_P2P_PEER_B_TOKEN");
}

#[test]
fn load_checked_fails_closed_on_invalid_p2p_static_config() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_P2P__INBOUND_TOKEN_ENV", None),
        ("DEVE_P2P_MESH_PEER_0_LABEL", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[p2p]
enabled = true
inbound_token_env = ""

[[p2p.peers]]
label = "peer-b"
peer_id = "peer-b-id"
repo_id = "11111111-1111-1111-1111-111111111111"
ws_url = "http://peer-b:3001/ws"
auth_token_env = ""
"#,
    )
    .expect("bad p2p config");

    let err = Config::load_checked().expect_err("invalid p2p static config");

    assert!(err.to_string().contains("p2p.inbound_token_env"));
}

#[test]
fn load_checked_fails_closed_on_invalid_p2p_peer_ws_url() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[("DEVE_P2P_MESH_PEER_0_LABEL", None)]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[[p2p.peers]]
label = "peer-b"
peer_id = "peer-b-id"
repo_id = "11111111-1111-1111-1111-111111111111"
ws_url = "http://peer-b:3001/ws"
auth_token_env = "DEVE_P2P_PEER_B_TOKEN"
"#,
    )
    .expect("bad p2p peer config");

    let err = Config::load_checked().expect_err("invalid p2p peer ws_url");

    assert!(err.to_string().contains("p2p.peers[0].ws_url"));
}

#[test]
fn load_checked_fails_closed_on_invalid_p2p_peer_repo_id() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[("DEVE_P2P_MESH_PEER_0_LABEL", None)]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[[p2p.peers]]
label = "peer-b"
peer_id = "peer-b-id"
repo_id = "not-a-uuid"
ws_url = "ws://peer-b:3001/ws"
auth_token_env = "DEVE_P2P_PEER_B_TOKEN"
"#,
    )
    .expect("bad p2p peer config");

    let err = Config::load_checked().expect_err("invalid p2p peer repo_id");

    assert!(err.to_string().contains("p2p.peers[0].repo_id"));
}

#[test]
fn load_checked_fails_closed_on_empty_p2p_peer_auth_token_env() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[("DEVE_P2P_MESH_PEER_0_LABEL", None)]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[[p2p.peers]]
label = "peer-b"
peer_id = "peer-b-id"
repo_id = "11111111-1111-1111-1111-111111111111"
ws_url = "ws://peer-b:3001/ws"
auth_token_env = ""
"#,
    )
    .expect("bad p2p peer config");

    let err = Config::load_checked().expect_err("empty p2p peer auth_token_env");

    assert!(err.to_string().contains("p2p.peers[0].auth_token_env"));
}

#[test]
fn load_checked_fails_closed_on_duplicate_p2p_peer_identity_tuple() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_P2P_MESH_PEER_0_LABEL", None),
        ("DEVE_P2P_MESH_PEER_1_LABEL", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[[p2p.peers]]
label = "edge-a"
peer_id = "peer-b-id"
repo_id = "11111111-1111-1111-1111-111111111111"
ws_url = "ws://peer-b:3001/ws"
auth_token_env = "DEVE_P2P_PEER_B_TOKEN"

[[p2p.peers]]
label = "edge-b"
peer_id = "peer-b-id"
repo_id = "11111111-1111-1111-1111-111111111111"
ws_url = "ws://peer-b:3001/ws"
auth_token_env = "DEVE_P2P_PEER_B_TOKEN_ALT"
"#,
    )
    .expect("duplicate p2p peer config");

    let err = Config::load_checked().expect_err("duplicate p2p peer tuple must fail closed");

    assert!(err.to_string().contains("p2p.peers[1]"));
    assert!(err.to_string().contains("duplicate"));
}

#[test]
fn load_checked_rejects_sparse_p2p_peer_env_alias_indices() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_P2P__ENABLED", Some("true")),
        ("DEVE_P2P_MESH_PEER_0_LABEL", None),
        ("DEVE_P2P_MESH_PEER_1_LABEL", Some("peer-b")),
        ("DEVE_P2P_MESH_PEER_1_PEER_ID", Some("peer-b-id")),
        (
            "DEVE_P2P_MESH_PEER_1_REPO_ID",
            Some("11111111-1111-1111-1111-111111111111"),
        ),
        ("DEVE_P2P_MESH_PEER_1_WS_URL", Some("ws://peer-b:3001/ws")),
        (
            "DEVE_P2P_MESH_PEER_1_AUTH_TOKEN_ENV",
            Some("DEVE_P2P_PEER_B_TOKEN"),
        ),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("sparse p2p peer env index must fail closed");

    assert!(err.to_string().contains("P2P peer environment indices"));
    assert!(err.to_string().contains("1"));
}
