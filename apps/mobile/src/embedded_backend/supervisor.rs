//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!   - 07_network#native-full-peer-runtime
//!
//! Owns one embedded runtime and replaceable loopback transport generations.

#![cfg_attr(not(mobile), allow(dead_code))]

use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use deve_cli::native_runtime::{NativeEmbeddedServerRuntime, NativeEmbeddedTransportRuntime};
use tokio::sync::Notify;

use super::generation::{
    await_transport_task, backend_requires_restart, prepare_transport,
    probe_existing_transport_async, probe_transport_async, probe_transport_initial,
    spawn_transport, started_shell, stop_transport,
};
use super::supervisor_types::{
    BackendGeneration, MobileEmbeddedBackendResume, MobileEmbeddedBackendServiceState,
    MobileEmbeddedBackendSupervisorSnapshot, ensure_current_transition, next_transition_token,
    record_error, snapshot_from_generation,
};
use super::{
    MobileEmbeddedBackendBootstrap, MobileEmbeddedBackendError, MobileEmbeddedBackendPlan,
};
use crate::{MobileLifecycleEvent, MobileShell};

pub const MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(mobile)]
#[path = "supervisor_webview.rs"]
mod webview;

#[path = "supervisor_shutdown.rs"]
mod shutdown;
#[cfg(mobile)]
use shutdown::ResumeActivity;

struct ResumePlan {
    transport: NativeEmbeddedTransportRuntime,
    plan: MobileEmbeddedBackendPlan,
    native_session_cookie: super::cookie::MobileNativeSessionCookie,
    shell: MobileShell,
    session_generation: u64,
    transition_token: u64,
    restart: bool,
    old_task: Option<super::generation::BackendTask>,
    probe_cancel: Arc<AtomicBool>,
}

pub struct MobileEmbeddedBackendSupervisor {
    app_data_dir: PathBuf,
    inner: Mutex<BackendGeneration>,
    active_resumes: AtomicUsize,
    resumes_idle: Notify,
    resume_gate: tokio::sync::Mutex<()>,
}

impl fmt::Debug for MobileEmbeddedBackendSupervisor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MobileEmbeddedBackendSupervisor")
            .field("app_data_dir", &self.app_data_dir)
            .field("snapshot", &self.snapshot().ok())
            .finish()
    }
}

impl MobileEmbeddedBackendSupervisor {
    pub fn start(
        app_data_dir: impl Into<PathBuf>,
    ) -> Result<(Self, MobileEmbeddedBackendBootstrap), MobileEmbeddedBackendError> {
        let app_data_dir = app_data_dir.into();
        let mut prepared = prepare_transport(&app_data_dir)?;
        let runtime = tauri::async_runtime::block_on(NativeEmbeddedServerRuntime::initialize(
            &prepared.options,
        ))
        .map_err(|error| MobileEmbeddedBackendError::RuntimeInitializeFailed(error.to_string()))?;
        let transport = runtime.transport();
        let (task, shutdown_sender) = spawn_transport(&transport, &mut prepared);
        let shell = started_shell();
        let probed = match probe_transport_initial(
            prepared.plan.clone(),
            prepared.native_session_secret.clone(),
            shell,
        ) {
            Ok(probed) => probed,
            Err(error) => {
                let _ = shutdown_sender.send(());
                tauri::async_runtime::block_on(async {
                    let deadline =
                        tokio::time::Instant::now() + MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT;
                    let _ = await_transport_task(task, deadline).await;
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    let _ = runtime.shutdown(remaining).await;
                });
                return Err(error);
            }
        };
        let bootstrap = probed.bootstrap.clone();
        let native_session_cookie = bootstrap.script.native_session_cookie.clone();
        let supervisor = Self {
            app_data_dir,
            inner: Mutex::new(BackendGeneration {
                runtime: Some(runtime),
                plan: prepared.plan,
                native_session_cookie,
                task: Some(task),
                shutdown_sender: Some(shutdown_sender),
                transport_stopping: false,
                runtime_restart_required: false,
                probe_cancel: None,
                shell: probed.shell,
                session_generation: 1,
                transition_token: 0,
                service_state: MobileEmbeddedBackendServiceState::EndpointSessionReady,
                last_error: None,
                last_error_transition_token: None,
            }),
            active_resumes: AtomicUsize::new(0),
            resumes_idle: Notify::new(),
            resume_gate: tokio::sync::Mutex::new(()),
        };
        eprintln!("deve_mobile native embedded backend supervisor=started");
        Ok((supervisor, bootstrap))
    }

    pub fn snapshot(
        &self,
    ) -> Result<MobileEmbeddedBackendSupervisorSnapshot, MobileEmbeddedBackendError> {
        let inner = self.lock_inner()?;
        Ok(snapshot_from_generation(&inner))
    }

    pub fn suspend(&self) -> Result<u64, MobileEmbeddedBackendError> {
        let mut inner = self.lock_inner()?;
        if matches!(
            inner.service_state,
            MobileEmbeddedBackendServiceState::Stopping
                | MobileEmbeddedBackendServiceState::Stopped
        ) {
            return Err(MobileEmbeddedBackendError::LifecycleTransitionCancelled);
        }
        if inner.runtime_restart_required {
            return Err(MobileEmbeddedBackendError::RuntimeRestartRequired);
        }
        inner.transition_token = next_transition_token(inner.transition_token)?;
        if let Some(cancel) = inner.probe_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        inner
            .shell
            .handle_lifecycle_event(MobileLifecycleEvent::Suspended);
        inner.service_state = MobileEmbeddedBackendServiceState::BackgroundSuspended;
        inner.last_error = None;
        Ok(inner.transition_token)
    }

    #[cfg(debug_assertions)]
    pub(crate) fn stop_transport_for_lifecycle_smoke(
        &self,
    ) -> Result<(), MobileEmbeddedBackendError> {
        let mut inner = self.lock_inner()?;
        if matches!(
            inner.service_state,
            MobileEmbeddedBackendServiceState::Stopping
                | MobileEmbeddedBackendServiceState::Stopped
        ) {
            return Err(MobileEmbeddedBackendError::LifecycleTransitionCancelled);
        }
        if let Some(sender) = inner.shutdown_sender.take() {
            let _ = sender.send(());
        }
        inner.transport_stopping = true;
        Ok(())
    }

    async fn resume_transition(
        &self,
    ) -> Result<MobileEmbeddedBackendResume, MobileEmbeddedBackendError> {
        let plan = self.begin_resume()?;
        if plan.restart {
            self.resume_with_replacement(plan).await
        } else {
            self.resume_existing_transport(plan).await
        }
    }

    fn begin_resume(&self) -> Result<ResumePlan, MobileEmbeddedBackendError> {
        let mut inner = self.lock_inner()?;
        if matches!(
            inner.service_state,
            MobileEmbeddedBackendServiceState::Stopping
                | MobileEmbeddedBackendServiceState::Stopped
        ) {
            return Err(MobileEmbeddedBackendError::LifecycleTransitionCancelled);
        }
        if inner.runtime_restart_required {
            return Err(MobileEmbeddedBackendError::RuntimeRestartRequired);
        }
        inner.transition_token = next_transition_token(inner.transition_token)?;
        inner
            .shell
            .handle_lifecycle_event(MobileLifecycleEvent::Resumed);
        inner.service_state = MobileEmbeddedBackendServiceState::ForegroundReprobe;
        inner.last_error = None;
        if let Some(cancel) = inner.probe_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        let probe_cancel = Arc::new(AtomicBool::new(false));
        inner.probe_cancel = Some(probe_cancel.clone());
        let restart = inner.transport_stopping
            || inner.shutdown_sender.is_none()
            || backend_requires_restart(inner.task.as_ref());
        let mut old_task = None;
        if restart {
            if let Some(sender) = inner.shutdown_sender.take() {
                let _ = sender.send(());
            }
            inner.transport_stopping = true;
            old_task = inner.task.take();
        }
        let transport = inner
            .runtime
            .as_ref()
            .ok_or(MobileEmbeddedBackendError::RuntimeUnavailable)?
            .transport();
        let session_generation = if restart {
            inner
                .session_generation
                .checked_add(1)
                .ok_or(MobileEmbeddedBackendError::SessionGenerationOverflow)?
        } else {
            inner.session_generation
        };
        Ok(ResumePlan {
            transport,
            plan: inner.plan.clone(),
            native_session_cookie: inner.native_session_cookie.clone(),
            shell: inner.shell.clone(),
            session_generation,
            transition_token: inner.transition_token,
            restart,
            old_task,
            probe_cancel,
        })
    }

    async fn resume_existing_transport(
        &self,
        plan: ResumePlan,
    ) -> Result<MobileEmbeddedBackendResume, MobileEmbeddedBackendError> {
        let probed = probe_existing_transport_async(
            plan.plan.clone(),
            plan.native_session_cookie.clone(),
            plan.shell,
            plan.probe_cancel.clone(),
        )
        .await;
        let probed = match probed {
            Ok(probed) => probed,
            Err(error) => {
                self.record_error_if_current(plan.transition_token, &error)?;
                return Err(error);
            }
        };
        let mut inner = self.lock_inner()?;
        ensure_current_transition(&inner, plan.transition_token)?;
        inner.shell = probed.shell;
        inner.service_state = MobileEmbeddedBackendServiceState::EndpointSessionReady;
        inner.probe_cancel = None;
        inner.transport_stopping = false;
        inner.last_error = None;
        inner.last_error_transition_token = None;
        Ok(MobileEmbeddedBackendResume {
            #[cfg(mobile)]
            native_session_cookie: probed.bootstrap.script.native_session_cookie.clone(),
            #[cfg(mobile)]
            replacement_bootstrap: None,
            restarted: false,
            session_generation: plan.session_generation,
            transition_token: plan.transition_token,
        })
    }

    async fn resume_with_replacement(
        &self,
        plan: ResumePlan,
    ) -> Result<MobileEmbeddedBackendResume, MobileEmbeddedBackendError> {
        if let Some(task) = plan.old_task {
            let deadline = tokio::time::Instant::now() + MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT;
            match await_transport_task(task, deadline).await {
                Ok(()) => {}
                Err(MobileEmbeddedBackendError::BackendExitedAfterSessionRetirement(error)) => {
                    eprintln!(
                        "deve_mobile retired LocalBackend transport exited after clean session retirement: {error}"
                    );
                }
                Err(error) => {
                    self.record_retirement_failure(plan.transition_token, &error)?;
                    return Err(error);
                }
            }
        }
        let mut prepared = match prepare_transport(&self.app_data_dir) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.record_error_if_current(plan.transition_token, &error)?;
                return Err(error);
            }
        };
        let (task, shutdown_sender) = spawn_transport(&plan.transport, &mut prepared);
        let probed = probe_transport_async(
            prepared.plan.clone(),
            prepared.native_session_secret.clone(),
            started_shell(),
            plan.probe_cancel.clone(),
        )
        .await;
        let probed = match probed {
            Ok(probed) => probed,
            Err(error) => {
                stop_transport(task, shutdown_sender).await;
                self.record_error_if_current(plan.transition_token, &error)?;
                return Err(error);
            }
        };

        let mut candidate_task = Some(task);
        let mut candidate_shutdown = Some(shutdown_sender);
        let commit = match self.lock_inner() {
            Ok(mut inner) => match ensure_current_transition(&inner, plan.transition_token) {
                Err(error) => Err(error),
                Ok(()) => {
                    inner.plan = prepared.plan;
                    inner.native_session_cookie =
                        probed.bootstrap.script.native_session_cookie.clone();
                    inner.task = candidate_task.take();
                    inner.shutdown_sender = candidate_shutdown.take();
                    inner.transport_stopping = false;
                    inner.runtime_restart_required = false;
                    inner.probe_cancel = None;
                    inner.shell = probed.shell;
                    inner.session_generation = plan.session_generation;
                    inner.service_state = MobileEmbeddedBackendServiceState::EndpointSessionReady;
                    inner.last_error = None;
                    inner.last_error_transition_token = None;
                    Ok(MobileEmbeddedBackendResume {
                        #[cfg(mobile)]
                        native_session_cookie: probed
                            .bootstrap
                            .script
                            .native_session_cookie
                            .clone(),
                        #[cfg(mobile)]
                        replacement_bootstrap: Some(probed.bootstrap),
                        restarted: true,
                        session_generation: plan.session_generation,
                        transition_token: plan.transition_token,
                    })
                }
            },
            Err(error) => Err(error),
        };
        match commit {
            Ok(resumed) => Ok(resumed),
            Err(error) => {
                stop_transport(
                    candidate_task.expect("failed commit retains candidate task"),
                    candidate_shutdown.expect("failed commit retains candidate shutdown sender"),
                )
                .await;
                Err(error)
            }
        }
    }

    fn record_error_if_current(
        &self,
        transition_token: u64,
        error: &MobileEmbeddedBackendError,
    ) -> Result<(), MobileEmbeddedBackendError> {
        let mut inner = self.lock_inner()?;
        if inner.transition_token == transition_token
            && !matches!(
                inner.service_state,
                MobileEmbeddedBackendServiceState::Stopping
                    | MobileEmbeddedBackendServiceState::Stopped
            )
        {
            record_error(&mut inner, error);
            inner.transport_stopping = true;
            inner.probe_cancel = None;
            inner.last_error_transition_token = Some(transition_token);
        }
        Ok(())
    }

    fn record_retirement_failure(
        &self,
        transition_token: u64,
        error: &MobileEmbeddedBackendError,
    ) -> Result<(), MobileEmbeddedBackendError> {
        let mut inner = self.lock_inner()?;
        inner.runtime_restart_required = true;
        if !matches!(
            inner.service_state,
            MobileEmbeddedBackendServiceState::Stopping
                | MobileEmbeddedBackendServiceState::Stopped
        ) {
            record_error(&mut inner, error);
            inner.probe_cancel = None;
            inner.last_error_transition_token = Some(transition_token);
        }
        Ok(())
    }

    fn lock_inner(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BackendGeneration>, MobileEmbeddedBackendError> {
        self.inner
            .lock()
            .map_err(|_| MobileEmbeddedBackendError::SupervisorStatePoisoned)
    }
}

#[cfg(test)]
#[path = "supervisor_tests.rs"]
mod tests;
