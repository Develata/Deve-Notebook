use deve_core::git_bridge::{DEVE_GIT_EXECUTABLE_ENV, DEVE_GIT_EXECUTABLE_UNAVAILABLE_ENV};
use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, native_tauri_allowed_origins,
};

use crate::{
    DesktopLocalServiceEntrypointError, DesktopLocalServiceEntrypointPolicy,
    plan_desktop_local_service_entrypoint,
};

use super::input;

#[test]
fn desktop_local_service_entrypoint_builds_controlled_deve_cli_serve_spec() {
    let request = input();
    let plan = plan_desktop_local_service_entrypoint(
        DesktopLocalServiceEntrypointPolicy::opt_in_enabled(),
        request.clone(),
    )
    .expect("plan")
    .expect("enabled plan");
    let spec = plan.spawn_spec;

    assert_eq!(
        spec.executable.file_name().and_then(|name| name.to_str()),
        Some(if cfg!(windows) {
            "deve_cli.exe"
        } else {
            "deve_cli"
        })
    );
    assert_eq!(spec.executable.parent(), request.current_exe.parent());
    assert_eq!(spec.argv, ["serve", "--native-loopback", "--port", "39101"]);
    assert_eq!(spec.cwd, request.data_root);
    assert_eq!(spec.bind_hints.http_host, "127.0.0.1");
    assert_eq!(spec.bind_hints.http_port, Some(39101));
    assert_eq!(spec.bind_hints.ws_host, "127.0.0.1");
    assert_eq!(spec.bind_hints.ws_port, Some(39101));
    assert!(spec.env_allowlist.contains(&"DEVE_PROFILE".to_string()));
    assert!(spec.env_allowlist.contains(&"DEVE_LEDGER_DIR".to_string()));
    assert!(!spec.env_allowlist.contains(&"DEVE_VAULT_PATH".to_string()));
    assert!(
        spec.env_allowlist
            .contains(&NATIVE_SESSION_BOOTSTRAP_SECRET_ENV.to_string())
    );
    assert!(spec.env_allowlist.contains(&"AUTH_SECRET".to_string()));
    assert!(spec.env_allowlist.contains(&"AUTH_PASS".to_string()));
    assert!(spec.env_allowlist.contains(&"AUTH_USER".to_string()));
    assert!(spec.env_allowlist.contains(&"ALLOWED_ORIGINS".to_string()));
    assert!(spec.env_allowlist.contains(&"DEVE_PLUGIN_DIR".to_string()));
    assert!(!spec.env_allowlist.contains(&"PATH".to_string()));
    assert!(!spec.env_allowlist.contains(&"PATHEXT".to_string()));
    assert!(
        !spec
            .env_allowlist
            .contains(&"AUTH_ALLOW_ANONYMOUS_LOCALHOST".to_string())
    );
    let secret = spec
        .env
        .iter()
        .find(|binding| binding.key == NATIVE_SESSION_BOOTSTRAP_SECRET_ENV)
        .expect("native session secret");
    assert_eq!(secret.value.len(), 64);
    assert!(secret.value.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert!(!format!("{:?}", secret).contains(&secret.value));
    let auth_secret = spec
        .env
        .iter()
        .find(|binding| binding.key == "AUTH_SECRET")
        .expect("auth secret");
    assert_eq!(auth_secret.value.len(), 64);
    assert!(auth_secret.value.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert!(!format!("{:?}", auth_secret).contains(&auth_secret.value));
    let auth_pass = spec
        .env
        .iter()
        .find(|binding| binding.key == "AUTH_PASS")
        .expect("auth pass");
    assert!(auth_pass.value.starts_with("$argon2"));
    assert!(!format!("{:?}", auth_pass).contains(&auth_pass.value));
    assert_eq!(
        spec.env
            .iter()
            .find(|binding| binding.key == "AUTH_USER")
            .map(|binding| binding.value.as_str()),
        Some("native")
    );
    let expected_tauri_origins = native_tauri_allowed_origins().join(",");
    assert_eq!(
        spec.env
            .iter()
            .find(|binding| binding.key == "ALLOWED_ORIGINS")
            .map(|binding| binding.value.as_str()),
        Some(expected_tauri_origins.as_str())
    );
    assert_eq!(
        spec.env
            .iter()
            .find(|binding| binding.key == "DEVE_PLUGIN_DIR")
            .map(|binding| binding.value.as_str()),
        request
            .current_exe
            .parent()
            .map(|path| path.join("plugins"))
            .as_ref()
            .map(|path| path.to_string_lossy())
            .as_deref()
    );
    assert_eq!(spec.profile, "low-spec");
    if let Some(git_binding) = spec
        .env
        .iter()
        .find(|binding| binding.key == DEVE_GIT_EXECUTABLE_ENV)
    {
        let path = std::path::Path::new(&git_binding.value);
        assert!(path.is_absolute());
        assert!(path.is_file());
        assert_eq!(std::fs::canonicalize(path).ok().as_deref(), Some(path));
        assert!(
            spec.env_allowlist
                .contains(&DEVE_GIT_EXECUTABLE_ENV.to_string())
        );
    }
    let git_bound = spec
        .env
        .iter()
        .any(|binding| binding.key == DEVE_GIT_EXECUTABLE_ENV);
    let git_unavailable = spec
        .env
        .iter()
        .any(|binding| binding.key == DEVE_GIT_EXECUTABLE_UNAVAILABLE_ENV && binding.value == "1");
    assert_ne!(git_bound, git_unavailable);
    assert!(plan.health_probe_required_before_bootstrap);
    assert!(plan.session_handoff_required_before_bootstrap);
    assert!(!plan.opens_authority_write_path);
    assert!(plan.policy.native_policy().child_process_runtime_enabled);
    assert!(
        plan.policy
            .native_policy()
            .is_desktop_local_backend_default()
    );
    assert!(!plan.policy.native_policy().authority_writes_allowed);
}

#[test]
fn desktop_local_service_entrypoint_missing_executable_parent_fails_closed() {
    let mut request = input();
    request.current_exe = std::path::PathBuf::new();

    let error = plan_desktop_local_service_entrypoint(
        DesktopLocalServiceEntrypointPolicy::opt_in_enabled(),
        request,
    )
    .expect_err("missing parent fails closed");

    assert!(matches!(
        error,
        DesktopLocalServiceEntrypointError::MissingExecutableParent
    ));
}
