use std::path::PathBuf;

use deve_core::config::AppProfile;
use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, native_tauri_allowed_origins,
};

use crate::{
    DEVE_DESKTOP_LOCAL_SERVICE_ENV, DEVE_NATIVE_AUTHORITY_ENV, DesktopLocalServiceEntrypointError,
    DesktopLocalServiceEntrypointInput, DesktopLocalServiceEntrypointPolicy,
    desktop_local_service_entrypoint_policy_from_env, plan_desktop_local_service_entrypoint,
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
fn desktop_local_service_entrypoint_is_local_backend_by_default() {
    let plan = plan_desktop_local_service_entrypoint(
        DesktopLocalServiceEntrypointPolicy::local_backend_default(),
        input(),
    )
    .expect("plan");

    assert!(plan.is_some());
}

#[test]
fn desktop_local_service_entrypoint_env_defaults_to_local_backend_and_allows_explicit_disable() {
    let _lock = ENV_LOCK.lock().expect("env lock");

    {
        let _guard = EnvGuard::set(&[
            (DEVE_NATIVE_AUTHORITY_ENV, None),
            (DEVE_DESKTOP_LOCAL_SERVICE_ENV, None),
        ]);
        assert_eq!(
            desktop_local_service_entrypoint_policy_from_env().expect("policy"),
            DesktopLocalServiceEntrypointPolicy::local_backend_default()
        );
    }

    {
        let _guard = EnvGuard::set(&[
            (DEVE_NATIVE_AUTHORITY_ENV, Some("0")),
            (DEVE_DESKTOP_LOCAL_SERVICE_ENV, None),
        ]);
        assert_eq!(
            desktop_local_service_entrypoint_policy_from_env().expect("policy"),
            DesktopLocalServiceEntrypointPolicy::local_backend_default()
        );
    }

    {
        let _guard = EnvGuard::set(&[
            (DEVE_NATIVE_AUTHORITY_ENV, None),
            (DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("0")),
        ]);
        assert_eq!(
            desktop_local_service_entrypoint_policy_from_env().expect("policy"),
            DesktopLocalServiceEntrypointPolicy::disabled()
        );
    }
}

#[test]
fn desktop_local_service_entrypoint_env_rejects_invalid_opt_in_value() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(&[
        (DEVE_NATIVE_AUTHORITY_ENV, Some("maybe")),
        (DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("1")),
    ]);

    let error =
        desktop_local_service_entrypoint_policy_from_env().expect_err("invalid env fails closed");

    assert!(matches!(
        error,
        DesktopLocalServiceEntrypointError::InvalidOptInValue {
            env: DEVE_NATIVE_AUTHORITY_ENV,
            ..
        }
    ));
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

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvGuard {
    old: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(vars: &[(&'static str, Option<&str>)]) -> Self {
        let old = vars
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in vars {
            // SAFETY: tests serialize env mutation through ENV_LOCK and restore every key.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
        Self { old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.old.drain(..) {
            // SAFETY: EnvGuard owns restoration for keys it changed while ENV_LOCK is held.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
