//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-service-supervisor-contract
//!   - 08_ui_design_02_desktop#desktop-packaging-scaffold

use std::sync::Mutex;

use deve_core::native_adapter::NativeProcessRuntimeSnapshot;
use tauri::plugin::TauriPlugin;
use tauri::webview::Cookie;
use tauri::webview::cookie::SameSite;
use thiserror::Error;

use crate::{
    DesktopBootstrap, DesktopCommandProcessLauncher, DesktopLocalServiceBootstrapError,
    DesktopLocalServiceEntrypointError, DesktopLocalServiceRuntime, DesktopLoopbackHttpProbe,
    DesktopNativeSessionCookie, DesktopProcessLauncher, DesktopRecoveryBootstrap, DesktopShell,
    DesktopShellError, plan_desktop_local_service_entrypoint_from_env,
    run_desktop_local_service_bootstrap,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopTauriBootstrapScript {
    source: String,
    recovery: bool,
    session_bound: bool,
    opens_authority_write_path: bool,
    native_session_cookie: Option<DesktopNativeSessionCookie>,
}

#[derive(Debug)]
pub struct DesktopTauriLocalServiceBootstrap<L = DesktopCommandProcessLauncher> {
    pub(crate) script: DesktopTauriBootstrapScript,
    pub(crate) runtime: Option<DesktopLocalServiceRuntime<L>>,
}

#[derive(Debug)]
pub struct DesktopLocalServiceTauriState<L = DesktopCommandProcessLauncher> {
    runtime: Mutex<DesktopLocalServiceRuntime<L>>,
}

impl<L> DesktopLocalServiceTauriState<L> {
    pub fn new(runtime: DesktopLocalServiceRuntime<L>) -> Self {
        Self {
            runtime: Mutex::new(runtime),
        }
    }
}

impl<L: DesktopProcessLauncher> DesktopLocalServiceTauriState<L> {
    pub fn runtime_snapshot(&self) -> Option<NativeProcessRuntimeSnapshot> {
        self.runtime.lock().ok().map(|runtime| runtime.snapshot())
    }
}

#[derive(Debug, Error)]
pub enum DesktopTauriBootstrapError {
    #[error(transparent)]
    Entrypoint(#[from] DesktopLocalServiceEntrypointError),
    #[error(transparent)]
    LocalService(#[from] DesktopLocalServiceBootstrapError),
    #[error(transparent)]
    Shell(#[from] DesktopShellError),
    #[error("desktop Tauri bootstrap requires a bound session")]
    SessionNotBound,
    #[error("desktop Tauri bootstrap requires a native session cookie")]
    NativeSessionCookieRequired,
    #[error("desktop Tauri bootstrap source contains forbidden material: {marker}")]
    ForbiddenMaterial { marker: &'static str },
}

impl DesktopTauriBootstrapScript {
    pub(super) fn new(
        source: String,
        recovery: bool,
        session_bound: bool,
        native_session_cookie: Option<DesktopNativeSessionCookie>,
    ) -> Result<Self, DesktopTauriBootstrapError> {
        validate_tauri_bootstrap_source(&source)?;
        Ok(Self {
            source,
            recovery,
            session_bound,
            opens_authority_write_path: false,
            native_session_cookie,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn is_recovery(&self) -> bool {
        self.recovery
    }

    pub fn session_bound(&self) -> bool {
        self.session_bound
    }

    pub fn opens_authority_write_path(&self) -> bool {
        self.opens_authority_write_path
    }

    pub fn has_native_session_cookie(&self) -> bool {
        self.native_session_cookie.is_some()
    }
}

pub fn desktop_tauri_success_init_script(
    bootstrap: &DesktopBootstrap,
    native_session_cookie: Option<DesktopNativeSessionCookie>,
) -> Result<DesktopTauriBootstrapScript, DesktopTauriBootstrapError> {
    if !bootstrap.session_bound {
        return Err(DesktopTauriBootstrapError::SessionNotBound);
    }
    if native_session_cookie.is_none() {
        return Err(DesktopTauriBootstrapError::NativeSessionCookieRequired);
    }
    DesktopTauriBootstrapScript::new(
        bootstrap.script_source()?,
        false,
        true,
        native_session_cookie,
    )
}

pub fn desktop_tauri_recovery_init_script(
    recovery: DesktopRecoveryBootstrap,
) -> Result<DesktopTauriBootstrapScript, DesktopTauriBootstrapError> {
    DesktopTauriBootstrapScript::new(recovery.script_source()?, true, false, None)
}

pub fn desktop_tauri_service_offline_init_script()
-> Result<DesktopTauriBootstrapScript, DesktopTauriBootstrapError> {
    desktop_tauri_recovery_init_script(DesktopRecoveryBootstrap {
        service_state: "service_offline",
    })
}

pub fn desktop_tauri_session_invalid_init_script()
-> Result<DesktopTauriBootstrapScript, DesktopTauriBootstrapError> {
    desktop_tauri_recovery_init_script(DesktopRecoveryBootstrap {
        service_state: "session_invalid",
    })
}

pub fn desktop_tauri_bootstrap_plugin<R: tauri::Runtime>(
    script: &DesktopTauriBootstrapScript,
) -> TauriPlugin<R> {
    let native_session_cookie = script.native_session_cookie.clone();
    tauri::plugin::Builder::new("deve-native-bootstrap")
        .js_init_script(script.source.clone())
        .on_webview_ready(move |webview| {
            if let Some(cookie) = native_session_cookie.as_ref() {
                webview
                    .set_cookie(tauri_cookie_from_native_session(cookie))
                    .expect("desktop native session cookie install failed before bootstrap");
            }
        })
        .build()
}

pub fn desktop_tauri_local_service_bootstrap_from_env(
    timestamp_unix_ms: i64,
) -> Option<DesktopTauriLocalServiceBootstrap> {
    match try_desktop_tauri_local_service_bootstrap_from_env(timestamp_unix_ms) {
        Ok(result) => result,
        Err(DesktopTauriBootstrapError::LocalService(
            DesktopLocalServiceBootstrapError::SessionHandoffFailed
            | DesktopLocalServiceBootstrapError::MissingNativeSessionBootstrapSecret
            | DesktopLocalServiceBootstrapError::NativeSessionCookieInvalid,
        )) => desktop_tauri_session_invalid_init_script()
            .ok()
            .map(recovery_bootstrap),
        Err(_) => desktop_tauri_service_offline_init_script()
            .ok()
            .map(recovery_bootstrap),
    }
}

pub fn try_desktop_tauri_local_service_bootstrap_from_env(
    timestamp_unix_ms: i64,
) -> Result<Option<DesktopTauriLocalServiceBootstrap>, DesktopTauriBootstrapError> {
    let Some(plan) = plan_desktop_local_service_entrypoint_from_env()? else {
        return Ok(None);
    };

    let mut runtime = DesktopLocalServiceRuntime::with_launcher(
        plan.policy.native_policy(),
        plan.policy.max_restart_attempts,
        DesktopCommandProcessLauncher::default(),
    );
    let mut shell = DesktopShell::new();
    let mut probe = DesktopLoopbackHttpProbe::default();
    let mut session_handoff = DesktopLoopbackHttpProbe::default();
    let result = run_desktop_local_service_bootstrap(
        &plan,
        &mut runtime,
        &mut shell,
        &mut probe,
        &mut session_handoff,
        timestamp_unix_ms,
    )?;
    let script = desktop_tauri_success_init_script(
        &result.bootstrap,
        result.session_material.native_session_cookie().cloned(),
    )?;

    Ok(Some(DesktopTauriLocalServiceBootstrap {
        script,
        runtime: Some(runtime),
    }))
}

fn tauri_cookie_from_native_session(cookie: &DesktopNativeSessionCookie) -> Cookie<'static> {
    Cookie::build((cookie.name().to_string(), cookie.value().to_string()))
        .domain(cookie.domain().to_string())
        .path(cookie.path().to_string())
        .http_only(cookie.http_only())
        .same_site(tauri_same_site_from_native_session(cookie.same_site()))
        .secure(cookie.secure())
        .build()
}

fn tauri_same_site_from_native_session(same_site: &str) -> SameSite {
    match same_site.to_ascii_lowercase().as_str() {
        "none" => SameSite::None,
        "lax" => SameSite::Lax,
        _ => SameSite::Strict,
    }
}

fn recovery_bootstrap(
    script: DesktopTauriBootstrapScript,
) -> DesktopTauriLocalServiceBootstrap<DesktopCommandProcessLauncher> {
    DesktopTauriLocalServiceBootstrap {
        script,
        runtime: None,
    }
}

fn validate_tauri_bootstrap_source(source: &str) -> Result<(), DesktopTauriBootstrapError> {
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
            return Err(DesktopTauriBootstrapError::ForbiddenMaterial { marker });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SameSite, tauri_same_site_from_native_session};

    #[test]
    fn tauri_cookie_mapping_preserves_native_session_same_site_none() {
        assert_eq!(tauri_same_site_from_native_session("None"), SameSite::None);
    }
}
