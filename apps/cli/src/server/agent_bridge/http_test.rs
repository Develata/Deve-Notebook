use super::{http_backend_capabilities, init_from_config};
use axum::Router;
use axum::body::{self, Body};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use serde_json::Value;
use tokio::sync::Mutex;
use tower::ServiceExt;

static POLICY_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn backend_capabilities_http_reports_trusted_cli_fallback_to_native() {
    let _guard = POLICY_TEST_LOCK.lock().await;
    let _env = EnvVarGuard::remove("AGENT_CLI_PATH");
    let mut config = deve_core::config::Config::load();
    config.ai.mode = "trusted-cli".to_string();
    config.ai.native_enabled = true;
    config.ai.agent_bridge.enabled = false;
    config.ai.agent_bridge.trusted = false;
    init_from_config(&config);

    let json = get_capabilities_json().await;

    assert_eq!(json["native_available"], true);
    assert_eq!(json["trusted_cli_available"], false);
    assert_eq!(json["effective_backend"], "native");
    assert_eq!(json["effective_backend_reason"], "external agent disabled");
}

#[tokio::test]
async fn backend_capabilities_http_reports_trusted_cli_when_policy_is_satisfied() {
    let _guard = POLICY_TEST_LOCK.lock().await;
    let cli_path = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .into_owned();
    let _env = EnvVarGuard::set("AGENT_CLI_PATH", cli_path);
    let mut config = deve_core::config::Config::load();
    config.ai.mode = "trusted-cli".to_string();
    config.ai.native_enabled = true;
    config.ai.agent_bridge.enabled = true;
    config.ai.agent_bridge.trusted = true;
    init_from_config(&config);

    let json = get_capabilities_json().await;

    assert_eq!(json["native_available"], true);
    assert_eq!(json["trusted_cli_available"], true);
    assert_eq!(json["effective_backend"], "trusted-cli");
    assert!(json["effective_backend_reason"].is_null());
}

#[tokio::test]
async fn backend_capabilities_http_reports_none_when_native_is_disabled() {
    let _guard = POLICY_TEST_LOCK.lock().await;
    let _env = EnvVarGuard::remove("AGENT_CLI_PATH");
    let mut config = deve_core::config::Config::load();
    config.ai.mode = "native".to_string();
    config.ai.native_enabled = false;
    config.ai.agent_bridge.enabled = false;
    config.ai.agent_bridge.trusted = false;
    init_from_config(&config);

    let json = get_capabilities_json().await;

    assert_eq!(json["native_available"], false);
    assert_eq!(json["trusted_cli_available"], false);
    assert_eq!(json["effective_backend"], "none");
    assert_eq!(json["native_reason"], "native AI disabled by config");
}

#[tokio::test]
async fn backend_capabilities_http_does_not_promote_native_mode_to_trusted_cli() {
    let _guard = POLICY_TEST_LOCK.lock().await;
    let cli_path = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .into_owned();
    let _env = EnvVarGuard::set("AGENT_CLI_PATH", cli_path);
    let mut config = deve_core::config::Config::load();
    config.ai.mode = "native".to_string();
    config.ai.native_enabled = false;
    config.ai.agent_bridge.enabled = true;
    config.ai.agent_bridge.trusted = true;
    init_from_config(&config);

    let json = get_capabilities_json().await;

    assert_eq!(json["native_available"], false);
    assert_eq!(json["trusted_cli_available"], true);
    assert_eq!(json["effective_backend"], "none");
    assert_eq!(
        json["effective_backend_reason"],
        "native AI disabled by config"
    );
}

async fn get_capabilities_json() -> Value {
    let app = Router::new().route(
        "/api/ai/backend-capabilities",
        get(http_backend_capabilities),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/ai/backend-capabilities")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = body::to_bytes(response.into_body(), 4096)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: String) -> Self {
        let guard = Self {
            key,
            previous: std::env::var_os(key),
        };
        // Tests hold POLICY_TEST_LOCK, so this process-wide env mutation is serialized.
        unsafe {
            std::env::set_var(key, value);
        }
        guard
    }

    fn remove(key: &'static str) -> Self {
        let guard = Self {
            key,
            previous: std::env::var_os(key),
        };
        // Tests hold POLICY_TEST_LOCK, so this process-wide env mutation is serialized.
        unsafe {
            std::env::remove_var(key);
        }
        guard
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // Restore the process env while still inside the serialized test scope.
        unsafe {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
