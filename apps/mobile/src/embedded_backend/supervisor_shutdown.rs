//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!   - 07_network#native-full-peer-runtime
//!
//! Bounded supervisor shutdown and in-flight resume ownership.

use std::sync::atomic::Ordering;
use std::time::Duration;

use super::super::supervisor_types::{MobileEmbeddedBackendServiceState, next_transition_token};
use super::super::{MobileEmbeddedBackendError, supervisor_types::record_error};
use super::{MobileEmbeddedBackendSupervisor, await_transport_task};

impl MobileEmbeddedBackendSupervisor {
    pub async fn shutdown(&self, timeout: Duration) -> Result<(), MobileEmbeddedBackendError> {
        let deadline = tokio::time::Instant::now() + timeout;
        self.initial_webview_session_admission.cancel();
        let handoff_guard = tokio::time::timeout_at(deadline, self.webview_handoff_gate.lock())
            .await
            .map_err(|_| MobileEmbeddedBackendError::ShutdownTimeout)?;
        let mut result = Ok(());
        let (task, pending, runtime, shutdown_coordinator, effective_deadline) = {
            let mut inner = self.lock_inner()?;
            if inner.service_state == MobileEmbeddedBackendServiceState::Stopped {
                return Ok(());
            }
            if inner.service_state == MobileEmbeddedBackendServiceState::Stopping {
                return Err(MobileEmbeddedBackendError::ShutdownInProgress);
            }
            inner.transition_token = next_transition_token(inner.transition_token)?;
            inner.service_state = MobileEmbeddedBackendServiceState::Stopping;
            inner.last_error = None;
            let shutdown_coordinator = inner.shutdown_coordinator.clone();
            let effective_deadline = shutdown_coordinator.begin_until(deadline);
            if let Some(sender) = inner.shutdown_sender.take() {
                let _ = sender.send(());
            }
            if let Some(cancel) = inner.probe_cancel.take() {
                cancel.store(true, Ordering::Release);
            }
            inner.transport_stopping = true;
            (
                inner.task.take(),
                inner.pending_transport.take(),
                inner.runtime.take(),
                shutdown_coordinator,
                effective_deadline,
            )
        };
        drop(handoff_guard);

        if let Err(error) = self.wait_for_resumes(effective_deadline).await
            && result.is_ok()
        {
            result = Err(error);
        }
        if let Some(task) = task
            && let Err(error) = await_transport_task(task, effective_deadline).await
            && result.is_ok()
        {
            result = Err(error);
        }
        if let Some(pending) = pending {
            pending.shutdown_coordinator.begin_until(effective_deadline);
            let _ = pending.shutdown_sender.send(());
            if let Err(error) = await_transport_task(pending.task, effective_deadline).await
                && result.is_ok()
            {
                result = Err(error);
            }
        }
        if let Some(runtime) = runtime
            && let Err(error) = runtime
                .shutdown_with_coordinator(
                    effective_deadline.saturating_duration_since(tokio::time::Instant::now()),
                    &shutdown_coordinator,
                )
                .await
            && result.is_ok()
        {
            result = Err(MobileEmbeddedBackendError::RuntimeShutdownFailed(
                error.to_string(),
            ));
        }

        let mut inner = self.lock_inner()?;
        match &result {
            Ok(()) => {
                inner.service_state = MobileEmbeddedBackendServiceState::Stopped;
                inner.last_error = None;
                inner.last_error_transition_token = None;
            }
            Err(error) => record_error(&mut inner, error),
        }
        result
    }

    async fn wait_for_resumes(
        &self,
        deadline: tokio::time::Instant,
    ) -> Result<(), MobileEmbeddedBackendError> {
        loop {
            let notified = self.resumes_idle.notified();
            if self.active_resumes.load(Ordering::Acquire) == 0 {
                return Ok(());
            }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                return Err(MobileEmbeddedBackendError::ShutdownTimeout);
            }
        }
    }
}

impl Drop for MobileEmbeddedBackendSupervisor {
    fn drop(&mut self) {
        self.initial_webview_session_admission.cancel();
        let inner = match self.inner.get_mut() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner.transition_token = inner.transition_token.saturating_add(1);
        if let Some(sender) = inner.shutdown_sender.take() {
            let _ = sender.send(());
        }
        if let Some(cancel) = inner.probe_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        if let Some(task) = inner.task.take() {
            task.abort();
        }
        if let Some(pending) = inner.pending_transport.take() {
            pending.shutdown_coordinator.begin(Duration::ZERO);
            let _ = pending.shutdown_sender.send(());
            pending.task.abort();
        }
        drop(inner.runtime.take());
    }
}

#[cfg(mobile)]
pub(super) struct ResumeActivity<'a> {
    supervisor: &'a MobileEmbeddedBackendSupervisor,
}

#[cfg(mobile)]
impl<'a> ResumeActivity<'a> {
    pub(super) fn new(supervisor: &'a MobileEmbeddedBackendSupervisor) -> Self {
        supervisor.active_resumes.fetch_add(1, Ordering::AcqRel);
        Self { supervisor }
    }
}

#[cfg(mobile)]
impl Drop for ResumeActivity<'_> {
    fn drop(&mut self) {
        if self
            .supervisor
            .active_resumes
            .fetch_sub(1, Ordering::AcqRel)
            == 1
        {
            self.supervisor.resumes_idle.notify_waiters();
        }
    }
}
