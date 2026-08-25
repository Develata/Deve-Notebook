//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!   - 07_network#native-full-peer-runtime
//!
//! Replaceable loopback transport/session generation adapter.

#![cfg_attr(not(mobile), allow(dead_code))]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use deve_cli::native_runtime::{
    NativeEmbeddedTransportRuntime, NativeLocalBackendOptions, NativeLoopbackListener,
    NativeRuntimeShutdownCoordinator, bind_native_loopback_listener,
};
use deve_core::config::AppProfile;
use tauri::async_runtime::JoinHandle;
use tokio::sync::oneshot;

use super::cookie::MobileNativeSessionCookie;
use super::http::MobileLoopbackHttpProbe;
use super::{
    MobileEmbeddedBackendBootstrap, MobileEmbeddedBackendError, MobileEmbeddedBackendPlan,
    MobileNativeAuthMaterial, mobile_embedded_backend_script, plan_mobile_embedded_backend,
};
use crate::{MobileSessionMaterial, MobileShell};

pub(super) type BackendTask = JoinHandle<Result<(), BackendTaskFailure>>;
const ABORT_JOIN_GRACE: std::time::Duration = std::time::Duration::from_millis(250);

#[derive(Debug)]
pub(super) struct BackendTaskFailure {
    pub(super) message: String,
    pub(super) sessions_retired: bool,
}

pub(super) struct PreparedTransport {
    pub(super) plan: MobileEmbeddedBackendPlan,
    pub(super) native_session_secret: String,
    pub(super) options: NativeLocalBackendOptions,
    listener: Option<NativeLoopbackListener>,
}

pub(super) struct ProbedTransport {
    pub(super) shell: MobileShell,
    pub(super) bootstrap: MobileEmbeddedBackendBootstrap,
}

#[derive(Debug)]
pub(super) enum TransportAdmissionFailure {
    Startup(MobileEmbeddedBackendError),
    SessionHandoff(MobileEmbeddedBackendError),
}

impl TransportAdmissionFailure {
    pub(super) fn into_error(self) -> MobileEmbeddedBackendError {
        match self {
            Self::Startup(error) | Self::SessionHandoff(error) => error,
        }
    }

    pub(super) fn is_retryable_startup(&self) -> bool {
        matches!(
            self,
            Self::Startup(
                MobileEmbeddedBackendError::ProbeIo(_)
                    | MobileEmbeddedBackendError::ProbeHttpStatus { status: 500..=599 }
            )
        )
    }
}

pub(super) fn prepare_transport(
    app_data_dir: &Path,
) -> Result<PreparedTransport, MobileEmbeddedBackendError> {
    let listener = bind_native_loopback_listener(None)
        .map_err(MobileEmbeddedBackendError::PortAllocationFailed)?;
    let plan = plan_mobile_embedded_backend(app_data_dir.to_path_buf(), listener.port())?;
    let auth = MobileNativeAuthMaterial::generate()?;
    let native_session_secret = auth.native_session_secret.clone();
    let mut options = NativeLocalBackendOptions::new(plan.app_data_dir.clone(), plan.port)
        .with_auth_material(auth.into_native_loopback_auth_material());
    options.profile = AppProfile::Standard;
    options.session_bound = false;
    options.prewarm_enabled = false;
    Ok(PreparedTransport {
        plan,
        native_session_secret,
        options,
        listener: Some(listener),
    })
}

pub(super) fn spawn_transport(
    runtime: &NativeEmbeddedTransportRuntime,
    prepared: &mut PreparedTransport,
) -> (
    BackendTask,
    oneshot::Sender<()>,
    NativeRuntimeShutdownCoordinator,
) {
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let shutdown_coordinator = NativeRuntimeShutdownCoordinator::new();
    let runtime = runtime.clone();
    let transport_shutdown_coordinator = shutdown_coordinator.clone();
    let options = prepared.options.clone();
    let listener = prepared
        .listener
        .take()
        .expect("prepared loopback listener is consumed exactly once");
    let task = tauri::async_runtime::spawn(async move {
        runtime
            .serve_with_listener_until_shutdown_with_coordinator(
                options,
                listener,
                async move {
                    let _ = shutdown_receiver.await;
                },
                transport_shutdown_coordinator,
            )
            .await
            .map_err(|error| BackendTaskFailure {
                sessions_retired: error.sessions_retired(),
                message: error.to_string(),
            })
    });
    (task, shutdown_sender, shutdown_coordinator)
}

pub(super) fn started_shell() -> MobileShell {
    let mut shell = MobileShell::new();
    shell.start_service();
    shell
}

fn probe_transport(
    plan: MobileEmbeddedBackendPlan,
    native_session_secret: String,
    session_install_id: String,
    mut shell: MobileShell,
    cancelled: Option<&AtomicBool>,
) -> Result<ProbedTransport, TransportAdmissionFailure> {
    let probe = MobileLoopbackHttpProbe::default();
    let mut endpoint = probe
        .probe_node_role(&plan, cancelled)
        .map_err(TransportAdmissionFailure::Startup)?;
    let cookie = probe
        .bind_native_session(&plan, &endpoint, &native_session_secret)
        .map_err(TransportAdmissionFailure::SessionHandoff)?;
    endpoint.session_bound = true;
    shell
        .bind_endpoint(endpoint)
        .map_err(MobileEmbeddedBackendError::from)
        .map_err(TransportAdmissionFailure::SessionHandoff)?;
    shell
        .bind_session(MobileSessionMaterial::bound())
        .map_err(MobileEmbeddedBackendError::from)
        .map_err(TransportAdmissionFailure::SessionHandoff)?;
    let bootstrap = shell
        .bootstrap_for_web()
        .map_err(MobileEmbeddedBackendError::from)
        .map_err(TransportAdmissionFailure::SessionHandoff)?;
    let script = mobile_embedded_backend_script(bootstrap, cookie, &session_install_id)
        .map_err(TransportAdmissionFailure::SessionHandoff)?;
    Ok(ProbedTransport {
        shell,
        bootstrap: MobileEmbeddedBackendBootstrap { plan, script },
    })
}

pub(super) async fn probe_replacement_transport_async(
    plan: MobileEmbeddedBackendPlan,
    native_session_secret: String,
    session_install_id: String,
    shell: MobileShell,
    cancelled: Arc<AtomicBool>,
) -> Result<ProbedTransport, TransportAdmissionFailure> {
    tokio::task::spawn_blocking(move || {
        probe_transport(
            plan,
            native_session_secret,
            session_install_id,
            shell,
            Some(&cancelled),
        )
    })
    .await
    .map_err(|error| {
        TransportAdmissionFailure::Startup(MobileEmbeddedBackendError::TaskJoinFailed(
            error.to_string(),
        ))
    })?
}

pub(super) fn probe_transport_initial(
    plan: MobileEmbeddedBackendPlan,
    native_session_secret: String,
    session_install_id: String,
    shell: MobileShell,
) -> Result<ProbedTransport, MobileEmbeddedBackendError> {
    probe_transport(plan, native_session_secret, session_install_id, shell, None)
        .map_err(TransportAdmissionFailure::into_error)
}

pub(super) async fn probe_existing_transport_async(
    plan: MobileEmbeddedBackendPlan,
    native_session_cookie: MobileNativeSessionCookie,
    session_install_id: String,
    shell: MobileShell,
    cancelled: Arc<AtomicBool>,
) -> Result<ProbedTransport, MobileEmbeddedBackendError> {
    tokio::task::spawn_blocking(move || {
        let probe = MobileLoopbackHttpProbe::default();
        let mut endpoint = probe.probe_node_role(&plan, Some(&cancelled))?;
        probe.validate_native_session(&plan, &native_session_cookie)?;
        endpoint.session_bound = true;
        let mut shell = shell;
        shell.bind_endpoint(endpoint)?;
        shell.bind_session(MobileSessionMaterial::bound())?;
        let bootstrap = shell.bootstrap_for_web()?;
        let script =
            mobile_embedded_backend_script(bootstrap, native_session_cookie, &session_install_id)?;
        Ok(ProbedTransport {
            shell,
            bootstrap: MobileEmbeddedBackendBootstrap { plan, script },
        })
    })
    .await
    .map_err(|error| MobileEmbeddedBackendError::TaskJoinFailed(error.to_string()))?
}

pub(super) async fn await_transport_task(
    mut task: BackendTask,
    deadline: tokio::time::Instant,
) -> Result<(), MobileEmbeddedBackendError> {
    match tokio::time::timeout_at(deadline, &mut task).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(error))) if error.sessions_retired => {
            Err(MobileEmbeddedBackendError::BackendExitedAfterSessionRetirement(error.message))
        }
        Ok(Ok(Err(error))) => Err(MobileEmbeddedBackendError::BackendExited(error.message)),
        Ok(Err(error)) => Err(MobileEmbeddedBackendError::TaskJoinFailed(
            error.to_string(),
        )),
        Err(_) => {
            task.abort();
            let _ = tokio::time::timeout(ABORT_JOIN_GRACE, task).await;
            Err(MobileEmbeddedBackendError::ShutdownTimeout)
        }
    }
}

pub(super) async fn stop_transport(
    task: BackendTask,
    shutdown_sender: oneshot::Sender<()>,
    shutdown_coordinator: &NativeRuntimeShutdownCoordinator,
) -> Result<(), MobileEmbeddedBackendError> {
    let deadline = shutdown_coordinator.begin(super::MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT);
    let _ = shutdown_sender.send(());
    await_transport_task(task, deadline).await
}

pub(super) fn backend_requires_restart(task: Option<&BackendTask>) -> bool {
    task.is_none_or(|task| task.inner().is_finished())
}

#[cfg(test)]
mod admission_tests {
    use super::*;

    #[test]
    fn replacement_retry_classification_stops_at_session_handoff() {
        assert!(
            TransportAdmissionFailure::Startup(MobileEmbeddedBackendError::ProbeIo(
                std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "test")
            ))
            .is_retryable_startup()
        );
        assert!(
            TransportAdmissionFailure::Startup(MobileEmbeddedBackendError::ProbeHttpStatus {
                status: 503
            })
            .is_retryable_startup()
        );
        assert!(
            !TransportAdmissionFailure::Startup(MobileEmbeddedBackendError::ProbeInvalidResponse)
                .is_retryable_startup()
        );
        assert!(
            !TransportAdmissionFailure::SessionHandoff(
                MobileEmbeddedBackendError::NativeSessionHandoffFailed
            )
            .is_retryable_startup()
        );
    }
}
