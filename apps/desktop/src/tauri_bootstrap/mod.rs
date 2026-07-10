//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-service-supervisor-contract
//!   - 11_ui_design/02_desktop#desktop-packaging-scaffold

use std::sync::Mutex;

use deve_core::native_adapter::{
    NativeAdapterError, NativeProcessRuntimeSnapshot, NativeRemoteTarget,
    validate_native_remote_target,
};
use tauri::plugin::TauriPlugin;
use thiserror::Error;

use crate::{
    DesktopBootstrap, DesktopCommandProcessLauncher, DesktopLocalServiceBootstrapError,
    DesktopLocalServiceEntrypointError, DesktopLocalServiceEntrypointPolicy,
    DesktopLocalServiceRuntime, DesktopLoopbackHttpProbe, DesktopNativeSessionCookie,
    DesktopProcessLauncher, DesktopProcessRuntimeError, DesktopRecoveryBootstrap, DesktopShell,
    DesktopShellError, desktop_local_service_entrypoint_policy_from_env,
    ensure_desktop_local_service_data_root,
    plan_desktop_local_service_entrypoint_for_current_process, run_desktop_local_service_bootstrap,
};

mod cookie;

use cookie::{tauri_cookie_from_native_session, validate_tauri_bootstrap_source};

const DESKTOP_LOCAL_BACKEND_PORT_ATTEMPTS: usize = 3;
pub const DEVE_NATIVE_REMOTE_URL_ENV: &str = "DEVE_NATIVE_REMOTE_URL";

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
pub struct DesktopLocalServiceTauriState<L: DesktopProcessLauncher = DesktopCommandProcessLauncher>
{
    runtime: Mutex<DesktopLocalServiceRuntime<L>>,
}

impl<L: DesktopProcessLauncher> DesktopLocalServiceTauriState<L> {
    pub fn new(runtime: DesktopLocalServiceRuntime<L>) -> Self {
        Self {
            runtime: Mutex::new(runtime),
        }
    }

    pub fn runtime_snapshot(&self) -> Option<NativeProcessRuntimeSnapshot> {
        self.runtime.lock().ok().map(|runtime| runtime.snapshot())
    }

    pub fn stop(
        &self,
        timestamp_unix_ms: i64,
    ) -> Result<NativeProcessRuntimeSnapshot, DesktopProcessRuntimeError> {
        let mut runtime =
            self.runtime
                .lock()
                .map_err(|_| DesktopProcessRuntimeError::StopFailed {
                    source: std::io::Error::other(
                        "desktop local service runtime state is poisoned",
                    ),
                })?;
        runtime.stop(timestamp_unix_ms)
    }
}

impl<L: DesktopProcessLauncher> Drop for DesktopLocalServiceTauriState<L> {
    fn drop(&mut self) {
        if let Ok(runtime) = self.runtime.get_mut()
            && runtime.snapshot().handle.is_some()
        {
            let _ = runtime.stop(0);
        }
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
    #[error(transparent)]
    RemoteTarget(#[from] NativeAdapterError),
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

pub fn desktop_tauri_remote_browser_init_script(
    target: &NativeRemoteTarget,
) -> Result<DesktopTauriBootstrapScript, DesktopTauriBootstrapError> {
    validate_native_remote_target(target)?;
    let origin = serde_json::to_string(&target.https_origin)
        .expect("serializing a validated HTTPS origin string cannot fail");
    DesktopTauriBootstrapScript::new(
        format!(
            "(()=>{{const target=new URL({origin}).origin;if(window.top===window&&window.location.origin!==target){{window.location.replace(target);}}}})();"
        ),
        false,
        false,
        None,
    )
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
        Err(error) => local_service_recovery_bootstrap(error),
    }
}

pub fn desktop_tauri_local_service_bootstrap_with_policy(
    timestamp_unix_ms: i64,
    policy: DesktopLocalServiceEntrypointPolicy,
) -> Option<DesktopTauriLocalServiceBootstrap> {
    match try_desktop_tauri_local_service_bootstrap_with_policy(timestamp_unix_ms, policy) {
        Ok(result) => result,
        Err(error) => local_service_recovery_bootstrap(error),
    }
}

pub fn desktop_tauri_remote_browser_bootstrap_from_env()
-> Result<Option<DesktopTauriBootstrapScript>, DesktopTauriBootstrapError> {
    let Some(value) = std::env::var_os(DEVE_NATIVE_REMOTE_URL_ENV) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    desktop_tauri_remote_browser_bootstrap_from_origin(&value.to_string_lossy()).map(Some)
}

pub fn desktop_tauri_remote_browser_bootstrap_from_origin(
    https_origin: &str,
) -> Result<DesktopTauriBootstrapScript, DesktopTauriBootstrapError> {
    desktop_tauri_remote_browser_init_script(&NativeRemoteTarget {
        https_origin: https_origin.to_string(),
    })
}

pub fn try_desktop_tauri_local_service_bootstrap_from_env(
    timestamp_unix_ms: i64,
) -> Result<Option<DesktopTauriLocalServiceBootstrap>, DesktopTauriBootstrapError> {
    let policy = desktop_local_service_entrypoint_policy_from_env()?;
    try_desktop_tauri_local_service_bootstrap_with_policy(timestamp_unix_ms, policy)
}

pub fn try_desktop_tauri_local_service_bootstrap_with_policy(
    timestamp_unix_ms: i64,
    policy: DesktopLocalServiceEntrypointPolicy,
) -> Result<Option<DesktopTauriLocalServiceBootstrap>, DesktopTauriBootstrapError> {
    for attempt in 0..DESKTOP_LOCAL_BACKEND_PORT_ATTEMPTS {
        let Some(plan) = plan_desktop_local_service_entrypoint_for_current_process(policy)? else {
            return Ok(None);
        };
        ensure_desktop_local_service_data_root(&plan.spawn_spec.cwd)?;

        let mut runtime = DesktopLocalServiceRuntime::with_launcher(
            plan.policy.native_policy(),
            plan.policy.max_restart_attempts,
            DesktopCommandProcessLauncher::default(),
        );
        let mut shell = DesktopShell::new();
        let mut probe = DesktopLoopbackHttpProbe::default();
        let mut session_handoff = DesktopLoopbackHttpProbe::default();
        match run_desktop_local_service_bootstrap(
            &plan,
            &mut runtime,
            &mut shell,
            &mut probe,
            &mut session_handoff,
            timestamp_unix_ms,
        ) {
            Ok(result) => {
                let script = desktop_tauri_success_init_script(
                    &result.bootstrap,
                    result.session_material.native_session_cookie().cloned(),
                )?;

                return Ok(Some(DesktopTauriLocalServiceBootstrap {
                    script,
                    runtime: Some(runtime),
                }));
            }
            Err(error)
                if attempt + 1 < DESKTOP_LOCAL_BACKEND_PORT_ATTEMPTS
                    && desktop_local_service_error_allows_port_replan(&error) =>
            {
                let _ = runtime.stop(timestamp_unix_ms.saturating_add(3));
                continue;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(None)
}

pub(crate) fn desktop_local_service_error_allows_port_replan(
    error: &DesktopLocalServiceBootstrapError,
) -> bool {
    match error {
        DesktopLocalServiceBootstrapError::Runtime(DesktopProcessRuntimeError::SpawnFailed {
            kind,
            ..
        }) => kind.retryable_by_default(),
        DesktopLocalServiceBootstrapError::HealthProbeFailed
        | DesktopLocalServiceBootstrapError::ProbeIo(_)
        | DesktopLocalServiceBootstrapError::ProbeHttpStatus { .. }
        | DesktopLocalServiceBootstrapError::ProbeResponseTooLarge
        | DesktopLocalServiceBootstrapError::ProbeInvalidResponse
        | DesktopLocalServiceBootstrapError::InvalidNodeRolePayload => true,
        DesktopLocalServiceBootstrapError::Runtime(_)
        | DesktopLocalServiceBootstrapError::Shell(_)
        | DesktopLocalServiceBootstrapError::SessionHandoffFailed
        | DesktopLocalServiceBootstrapError::InvalidProbeUrl
        | DesktopLocalServiceBootstrapError::InvalidEndpoint(_)
        | DesktopLocalServiceBootstrapError::MissingNativeSessionBootstrapSecret
        | DesktopLocalServiceBootstrapError::NativeSessionCookieInvalid => false,
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

fn local_service_recovery_bootstrap(
    error: DesktopTauriBootstrapError,
) -> Option<DesktopTauriLocalServiceBootstrap<DesktopCommandProcessLauncher>> {
    match error {
        DesktopTauriBootstrapError::LocalService(
            DesktopLocalServiceBootstrapError::SessionHandoffFailed
            | DesktopLocalServiceBootstrapError::MissingNativeSessionBootstrapSecret
            | DesktopLocalServiceBootstrapError::NativeSessionCookieInvalid,
        ) => desktop_tauri_session_invalid_init_script()
            .ok()
            .map(recovery_bootstrap),
        _ => desktop_tauri_service_offline_init_script()
            .ok()
            .map(recovery_bootstrap),
    }
}
