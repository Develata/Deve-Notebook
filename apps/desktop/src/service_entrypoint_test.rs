use std::path::PathBuf;

use deve_core::config::AppProfile;
use deve_core::native_adapter::NATIVE_SESSION_BOOTSTRAP_SECRET_ENV;

use crate::{
    DesktopLocalServiceEntrypointError, DesktopLocalServiceEntrypointInput,
    DesktopLocalServiceEntrypointPolicy, plan_desktop_local_service_entrypoint,
};

fn abs(path: &str) -> PathBuf {
    let root = std::env::current_dir().expect("current dir");
    root.join(path)
}

fn input() -> DesktopLocalServiceEntrypointInput {
    DesktopLocalServiceEntrypointInput {
        current_exe: abs("target/debug/deve_desktop"),
        data_root: abs("desktop-data"),
        port: 39101,
        profile: AppProfile::LowSpec,
    }
}

#[test]
fn desktop_local_service_entrypoint_is_disabled_without_opt_in() {
    let plan = plan_desktop_local_service_entrypoint(
        DesktopLocalServiceEntrypointPolicy::disabled(),
        input(),
    )
    .expect("plan");

    assert!(plan.is_none());
}

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
    assert!(spec.env_allowlist.contains(&"DEVE_VAULT_PATH".to_string()));
    assert!(
        spec.env_allowlist
            .contains(&NATIVE_SESSION_BOOTSTRAP_SECRET_ENV.to_string())
    );
    assert!(spec.env_allowlist.contains(&"AUTH_SECRET".to_string()));
    assert!(spec.env_allowlist.contains(&"AUTH_PASS".to_string()));
    assert!(spec.env_allowlist.contains(&"AUTH_USER".to_string()));
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
    assert_eq!(spec.profile, "low-spec");
    assert!(plan.health_probe_required_before_bootstrap);
    assert!(plan.session_handoff_required_before_bootstrap);
    assert!(!plan.opens_authority_write_path);
    assert!(plan.policy.native_policy().child_process_runtime_enabled);
    assert!(!plan.policy.native_policy().authority_writes_allowed);
}

#[test]
fn desktop_local_service_entrypoint_rejects_relative_data_root() {
    let mut request = input();
    request.data_root = PathBuf::from("relative-data");

    let error = plan_desktop_local_service_entrypoint(
        DesktopLocalServiceEntrypointPolicy::opt_in_enabled(),
        request,
    )
    .expect_err("relative paths fail closed");

    assert!(matches!(
        error,
        DesktopLocalServiceEntrypointError::InvalidSpawnSpec(_)
    ));
}

#[test]
fn desktop_local_service_entrypoint_missing_executable_parent_fails_closed() {
    let mut request = input();
    request.current_exe = PathBuf::new();

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
