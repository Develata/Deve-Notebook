//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!
//! Mobile LocalBackend assembly. This module starts the embedded backend and
//! performs native-session handoff; business authority stays in server/core.

use std::fmt;
use std::path::PathBuf;

use deve_cli::server::NativeLoopbackAuthMaterial;
use deve_core::native_adapter::{NativeAdapterError, native_tauri_allowed_origins};
use deve_core::security::auth::password;
use tauri::plugin::TauriPlugin;
use thiserror::Error;

use crate::{MobileBootstrap, MobileShellError};

#[cfg(target_os = "android")]
mod android_cookie;
#[cfg(any(target_os = "android", test))]
mod android_cookie_callback;
#[cfg(test)]
mod bootstrap_script_tests;
mod cookie;
mod generation;
mod http;
mod supervisor;
mod supervisor_types;

const WEBVIEW_BOOTSTRAP_INIT_SOURCE: &str = include_str!("webview_bootstrap_init.js");
#[cfg(any(target_os = "android", test))]
const ANDROID_INITIAL_SESSION_PREPARE_SOURCE: &str =
    include_str!("android_initial_session_prepare.js");

use cookie::MobileNativeSessionCookie;
pub use supervisor::{MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT, MobileEmbeddedBackendSupervisor};
pub use supervisor_types::{
    MobileEmbeddedBackendResume, MobileEmbeddedBackendServiceState,
    MobileEmbeddedBackendSupervisorSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileEmbeddedBackendPlan {
    pub app_data_dir: PathBuf,
    pub port: u16,
    pub http_base: String,
    pub ws_base: String,
    pub embedded_service_runtime_enabled: bool,
    pub opens_authority_write_path: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileEmbeddedBackendBootstrap {
    pub plan: MobileEmbeddedBackendPlan,
    pub script: MobileEmbeddedBackendScript,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileEmbeddedBackendScript {
    source: String,
    replacement_source: String,
    native_session_cookie: MobileNativeSessionCookie,
}

impl MobileEmbeddedBackendScript {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn has_native_session_cookie(&self) -> bool {
        self.native_session_cookie.has_value()
    }

    #[cfg(any(mobile, test))]
    pub(super) fn replacement_source(&self) -> &str {
        &self.replacement_source
    }
}

#[derive(Debug, Error)]
pub enum MobileEmbeddedBackendError {
    #[error("mobile LocalBackend app data dir must be absolute")]
    RelativeAppDataDir,
    #[error("mobile LocalBackend port must be non-zero")]
    InvalidPort,
    #[error("failed to allocate mobile LocalBackend loopback port")]
    PortAllocationFailed(#[source] std::io::Error),
    #[error("failed to generate mobile native session material")]
    SecretGenerationFailed,
    #[error("failed to generate mobile WebView session install identity")]
    SessionInstallIdGenerationFailed,
    #[error("failed to generate mobile native auth material")]
    AuthMaterialGenerationFailed,
    #[error("mobile LocalBackend probe URL is invalid")]
    InvalidProbeUrl,
    #[error("mobile LocalBackend endpoint is invalid")]
    InvalidEndpoint(#[from] NativeAdapterError),
    #[error("mobile LocalBackend probe HTTP status is not successful: {status}")]
    ProbeHttpStatus { status: u16 },
    #[error("mobile LocalBackend probe response is too large")]
    ProbeResponseTooLarge,
    #[error("mobile LocalBackend probe response is invalid")]
    ProbeInvalidResponse,
    #[error("mobile LocalBackend probe IO failed")]
    ProbeIo(#[source] std::io::Error),
    #[error("mobile native session handoff failed")]
    NativeSessionHandoffFailed,
    #[error("mobile native session cookie is invalid")]
    NativeSessionCookieInvalid,
    #[error("mobile LocalBackend bootstrap source contains forbidden material: {marker}")]
    ForbiddenMaterial { marker: &'static str },
    #[error(transparent)]
    Shell(#[from] MobileShellError),
    #[error("failed to serialize mobile LocalBackend bootstrap: {0}")]
    BootstrapSerialize(#[from] serde_json::Error),
    #[error("mobile LocalBackend supervisor state is poisoned")]
    SupervisorStatePoisoned,
    #[error("mobile LocalBackend session generation overflow")]
    SessionGenerationOverflow,
    #[error("mobile LocalBackend graceful shutdown timed out")]
    ShutdownTimeout,
    #[error("mobile LocalBackend task join failed: {0}")]
    TaskJoinFailed(String),
    #[error("mobile LocalBackend task exited with error: {0}")]
    BackendExited(String),
    #[error("mobile LocalBackend task exited after all transport sessions retired: {0}")]
    BackendExitedAfterSessionRetirement(String),
    #[error("mobile embedded authority runtime initialization failed: {0}")]
    RuntimeInitializeFailed(String),
    #[error("mobile embedded authority runtime is unavailable")]
    RuntimeUnavailable,
    #[error(
        "mobile embedded authority runtime requires app restart after failed transport retirement"
    )]
    RuntimeRestartRequired,
    #[error("mobile embedded authority runtime shutdown failed: {0}")]
    RuntimeShutdownFailed(String),
    #[error("mobile LocalBackend lifecycle transition was superseded or cancelled")]
    LifecycleTransitionCancelled,
    #[error("mobile LocalBackend shutdown is already in progress")]
    ShutdownInProgress,
    #[error("mobile LocalBackend WebView bootstrap install failed: {0}")]
    WebviewInstallFailed(String),
    #[error("mobile LocalBackend main WebView is unavailable")]
    WebviewUnavailable,
}

pub fn plan_mobile_embedded_backend(
    app_data_dir: impl Into<PathBuf>,
    port: u16,
) -> Result<MobileEmbeddedBackendPlan, MobileEmbeddedBackendError> {
    if port == 0 {
        return Err(MobileEmbeddedBackendError::InvalidPort);
    }
    let app_data_dir = app_data_dir.into();
    if !app_data_dir.is_absolute() {
        return Err(MobileEmbeddedBackendError::RelativeAppDataDir);
    }
    Ok(MobileEmbeddedBackendPlan {
        app_data_dir,
        port,
        http_base: format!("http://127.0.0.1:{port}"),
        ws_base: format!("ws://127.0.0.1:{port}"),
        embedded_service_runtime_enabled: true,
        opens_authority_write_path: false,
    })
}

pub fn mobile_embedded_backend_plugin<R: tauri::Runtime>(
    script: &MobileEmbeddedBackendScript,
) -> TauriPlugin<R> {
    let builder = tauri::plugin::Builder::<R, ()>::new("deve-mobile-local-backend")
        .js_init_script(script.source.clone());
    #[cfg(not(target_os = "android"))]
    let builder = {
        let native_session_cookie = script.native_session_cookie.clone();
        builder.on_webview_ready(move |webview| {
            if let Err(error) = webview.set_cookie(cookie::tauri_cookie_from_native_session(
                &native_session_cookie,
            )) {
                eprintln!("deve_mobile native session cookie install failed closed: {error}");
            }
        })
    };
    builder.build()
}

fn mobile_embedded_backend_script(
    bootstrap: MobileBootstrap,
    native_session_cookie: MobileNativeSessionCookie,
    session_install_id: &str,
) -> Result<MobileEmbeddedBackendScript, MobileEmbeddedBackendError> {
    let payload = serde_json::to_string(&bootstrap)?;
    let session_install_id_json = serde_json::to_string(session_install_id)?;
    let init = WEBVIEW_BOOTSTRAP_INIT_SOURCE;
    let source = format!(
        "(()=>{{const init={init};init(window,{payload},{session_install_id_json},false);}})();"
    );
    #[cfg(target_os = "android")]
    let source = android_initial_session_prepare_source(source, session_install_id)?;
    let replacement_source = format!(
        "(()=>{{const init={init};init(window,{payload},{session_install_id_json},true);}})();"
    );
    validate_mobile_embedded_script_source(&source)?;
    validate_mobile_embedded_script_source(&replacement_source)?;
    Ok(MobileEmbeddedBackendScript {
        source,
        replacement_source,
        native_session_cookie,
    })
}

#[cfg(any(target_os = "android", test))]
fn android_initial_session_prepare_source(
    source: String,
    session_install_id: &str,
) -> Result<String, MobileEmbeddedBackendError> {
    let session_install_id = serde_json::to_string(session_install_id)?;
    let prepare = ANDROID_INITIAL_SESSION_PREPARE_SOURCE;
    Ok(format!(
        "{source}(()=>{{const prepare={prepare};prepare(window,{session_install_id});}})();"
    ))
}

fn validate_mobile_embedded_script_source(source: &str) -> Result<(), MobileEmbeddedBackendError> {
    let source_lower = source.to_ascii_lowercase();
    for marker in [
        "<script",
        "</script",
        "token",
        "secret",
        "localstorage",
        "location.href",
        "auth_pass",
        "auth_secret",
    ] {
        if source_lower.contains(marker) {
            return Err(MobileEmbeddedBackendError::ForbiddenMaterial { marker });
        }
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct MobileNativeAuthMaterial {
    pub(super) native_session_secret: String,
    auth_secret: String,
    auth_password_hash: String,
}

impl fmt::Debug for MobileNativeAuthMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MobileNativeAuthMaterial")
            .field("native_session_secret", &"<redacted>")
            .field("auth_secret", &"<redacted>")
            .field("auth_password_hash", &"<redacted>")
            .finish()
    }
}

impl MobileNativeAuthMaterial {
    pub(super) fn generate() -> Result<Self, MobileEmbeddedBackendError> {
        let native_session_secret = generate_secret()?;
        let auth_secret = generate_secret()?;
        let auth_password = generate_secret()?;
        let auth_password_hash = password::hash_password(&auth_password)
            .map_err(|_| MobileEmbeddedBackendError::AuthMaterialGenerationFailed)?;
        Ok(Self {
            native_session_secret,
            auth_secret,
            auth_password_hash,
        })
    }

    pub(super) fn into_native_loopback_auth_material(self) -> NativeLoopbackAuthMaterial {
        NativeLoopbackAuthMaterial::new(
            self.native_session_secret,
            self.auth_secret,
            "native",
            self.auth_password_hash,
            native_tauri_allowed_origins(),
        )
    }
}

fn generate_secret() -> Result<String, MobileEmbeddedBackendError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| MobileEmbeddedBackendError::SecretGenerationFailed)?;
    Ok(hex_encode(&bytes))
}

fn generate_session_install_id() -> Result<String, MobileEmbeddedBackendError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| MobileEmbeddedBackendError::SessionInstallIdGenerationFailed)?;
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_cli::native_runtime::bind_native_loopback_listener;

    #[test]
    fn mobile_embedded_backend_plan_is_local_authority_free_runtime() {
        let root = std::env::current_dir()
            .expect("cwd")
            .join("target/mobile-test-data");
        let plan = plan_mobile_embedded_backend(root.clone(), 40123).expect("plan");

        assert_eq!(plan.app_data_dir, root);
        assert_eq!(plan.http_base, "http://127.0.0.1:40123");
        assert_eq!(plan.ws_base, "ws://127.0.0.1:40123");
        assert!(plan.embedded_service_runtime_enabled);
        assert!(!plan.opens_authority_write_path);
    }

    #[test]
    fn mobile_embedded_backend_plan_rejects_relative_root_and_zero_port() {
        assert!(matches!(
            plan_mobile_embedded_backend("relative", 40123),
            Err(MobileEmbeddedBackendError::RelativeAppDataDir)
        ));
        assert!(matches!(
            plan_mobile_embedded_backend(
                std::env::current_dir()
                    .expect("cwd")
                    .join("target/mobile-test-data"),
                0
            ),
            Err(MobileEmbeddedBackendError::InvalidPort)
        ));
    }

    #[test]
    fn mobile_embedded_backend_script_exposes_endpoint_without_cookie_material() {
        let cookie = MobileNativeSessionCookie::from_set_cookie(
            "token=cookie-value; Path=/; HttpOnly; Secure; SameSite=None",
            "127.0.0.1",
        )
        .expect("cookie");
        let script = mobile_embedded_backend_script(
            MobileBootstrap {
                http_base: "http://127.0.0.1:40123".to_string(),
                ws_base: "ws://127.0.0.1:40123".to_string(),
                node_role: "main".to_string(),
                session_bound: true,
                capabilities: deve_core::native_adapter::NativeShellCapabilities::local_backend(),
            },
            cookie,
            "process-session-a",
        )
        .expect("script");

        assert!(script.has_native_session_cookie());
        assert!(
            script
                .source()
                .contains("root.__DEVE_NATIVE_BOOTSTRAP = current")
        );
        assert!(script.source().contains("root.sessionStorage.getItem"));
        assert!(
            script
                .replacement_source()
                .contains("root.sessionStorage.setItem")
        );
        assert!(script.source().contains("http://127.0.0.1:40123"));
        assert!(!script.source().contains("secret"));
        assert!(!script.source().contains("token"));
        assert!(!script.source().contains("cookie-value"));
    }

    #[test]
    fn mobile_embedded_auth_material_uses_runtime_launch_options() {
        let auth = MobileNativeAuthMaterial::generate().expect("auth material");
        let debug = format!("{auth:?}");
        assert!(!debug.contains(&auth.native_session_secret));
        assert!(!debug.contains(&auth.auth_secret));
        assert!(!debug.contains(&auth.auth_password_hash));

        let material = auth.into_native_loopback_auth_material();
        let expected_origins = native_tauri_allowed_origins();

        assert_eq!(material.allowed_origins(), expected_origins.as_slice());
        assert_eq!(
            material.auth_config().expect("auth config").username,
            "native"
        );
    }

    #[test]
    fn mobile_embedded_bootstrap_keeps_listener_port_as_single_backend_target() {
        let listener = bind_native_loopback_listener(None).expect("listener");
        let port = listener.port();
        let root = std::env::current_dir()
            .expect("cwd")
            .join("target/mobile-test-data");
        let plan = plan_mobile_embedded_backend(root, port).expect("plan");

        assert_eq!(plan.port, port);
        assert_eq!(plan.http_base, format!("http://127.0.0.1:{port}"));
        assert_eq!(plan.ws_base, format!("ws://127.0.0.1:{port}"));
        drop(listener);
    }
}
