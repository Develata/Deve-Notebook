//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 15_settings#native-ai-provider-settings

use super::{
    NativeLocalBackendOptions, bind_native_loopback_listener, bind_native_loopback_listener_exact,
    init_default_native_backend, load_native_plugins, native_ai_provider_settings_root,
};

#[test]
fn native_default_backend_starts_empty_without_creating_repo_authority() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (repo, layout) = init_default_native_backend(dir.path(), 8).expect("init native backend");

    assert_eq!(
        layout.app_data_dir,
        std::fs::canonicalize(dir.path()).expect("canonical")
    );
    assert!(layout.ledger_dir.join("local").is_dir());
    assert!(layout.projection_base.is_dir());
    assert!(layout.workspace_root.is_none());
    let summaries = repo
        .list_cataloged_local_repo_summaries()
        .expect("catalog listing");
    assert!(
        summaries.is_empty(),
        "native startup must not invent a repo"
    );
    assert!(
        std::fs::read_dir(layout.ledger_dir.join("local"))
            .expect("local dir")
            .all(|entry| entry
                .expect("entry")
                .path()
                .extension()
                .is_none_or(|ext| ext != "redb")),
        "native NoScope startup must not create local Redb"
    );

    drop(repo);
    let (repo, second_layout) =
        init_default_native_backend(dir.path(), 8).expect("reopen native backend");
    assert_eq!(second_layout.workspace_root, None);
    assert_eq!(
        repo.list_cataloged_local_repo_summaries()
            .expect("catalog listing after reopen")
            .len(),
        0
    );
}

#[test]
fn native_local_backend_options_default_to_local_runtime_contract() {
    let options = NativeLocalBackendOptions::new("native-data", 39111);

    assert_eq!(options.port, 39111);
    assert_eq!(options.snapshot_depth, 100);
    assert!(!options.session_bound);
    assert!(options.auth_material.is_none());
    assert!(options.prewarm_enabled);
    assert!(!options.p2p.enabled);
}

#[test]
fn native_ai_settings_root_is_platform_owned_without_changing_authority_root() {
    let app_data = std::path::Path::new("/private/app-data");

    assert_eq!(native_ai_provider_settings_root(app_data, false), app_data);
    assert_eq!(
        native_ai_provider_settings_root(app_data, true),
        app_data.join("files")
    );
}

#[test]
fn native_ai_builtin_loads_without_external_plugin_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let plugins = load_native_plugins(dir.path(), true).expect("native plugins");

    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].manifest().id, "ai-chat");
}

#[test]
fn native_ai_disabled_omits_builtin_runtime() {
    let dir = tempfile::tempdir().expect("tempdir");
    let plugins = load_native_plugins(dir.path(), false).expect("native plugins");

    assert!(plugins.is_empty());
}

#[test]
fn native_loopback_listener_falls_back_when_preferred_port_is_occupied() {
    let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("occupy port");
    let occupied_port = occupied.local_addr().expect("addr").port();

    let listener = bind_native_loopback_listener(Some(occupied_port)).expect("fallback listener");

    assert_ne!(listener.port(), occupied_port);
    assert!(listener.port() > 0);
}

#[test]
fn native_loopback_listener_exact_rejects_occupied_port() {
    let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("occupy port");
    let occupied_port = occupied.local_addr().expect("addr").port();

    let error = bind_native_loopback_listener_exact(occupied_port).expect_err("exact bind fails");

    assert_eq!(error.kind(), std::io::ErrorKind::AddrInUse);
}
