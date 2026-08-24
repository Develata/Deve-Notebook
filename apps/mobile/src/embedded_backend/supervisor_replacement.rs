//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!   - 07_network#native-full-peer-runtime
//!
//! Bounded fresh-listener admission for replacement transport generations.

use super::*;
use crate::embedded_backend::generation::{
    await_transport_task, probe_replacement_transport_async, stop_transport,
};

const REPLACEMENT_ADMISSION_MAX_ATTEMPTS: usize = 2;

fn retryable_prepare_failure(error: &MobileEmbeddedBackendError) -> bool {
    matches!(error, MobileEmbeddedBackendError::PortAllocationFailed(_))
}

fn replacement_checkpoint(
    category: &'static str,
    transition_token: u64,
    session_generation: u64,
    attempt: usize,
) {
    eprintln!(
        "{}",
        replacement_checkpoint_message(category, transition_token, session_generation, attempt)
    );
}

fn replacement_checkpoint_message(
    category: &'static str,
    transition_token: u64,
    session_generation: u64,
    attempt: usize,
) -> String {
    format!(
        "deve_mobile LocalBackend replacement checkpoint: {category} transition={transition_token} generation={session_generation} attempt={attempt}"
    )
}

impl MobileEmbeddedBackendSupervisor {
    pub(super) async fn resume_with_replacement(
        &self,
        mut plan: ResumePlan,
    ) -> Result<MobileEmbeddedBackendResume, MobileEmbeddedBackendError> {
        if let Some(task) = plan.old_task.take() {
            let deadline = tokio::time::Instant::now() + MOBILE_EMBEDDED_BACKEND_SHUTDOWN_TIMEOUT;
            match await_transport_task(task, deadline).await {
                Ok(())
                | Err(MobileEmbeddedBackendError::BackendExitedAfterSessionRetirement(_)) => {}
                Err(error) => {
                    self.record_retirement_failure(plan.transition_token, &error)?;
                    return Err(error);
                }
            }
        }

        for attempt in 1..=REPLACEMENT_ADMISSION_MAX_ATTEMPTS {
            {
                let inner = self.lock_inner()?;
                ensure_current_transition(&inner, plan.transition_token)?;
            }
            replacement_checkpoint(
                "android_local_backend_replacement_attempt",
                plan.transition_token,
                plan.session_generation,
                attempt,
            );

            let mut prepared = match prepare_transport(&self.app_data_dir) {
                Ok(prepared) => prepared,
                Err(error)
                    if attempt < REPLACEMENT_ADMISSION_MAX_ATTEMPTS
                        && retryable_prepare_failure(&error) =>
                {
                    replacement_checkpoint(
                        "android_local_backend_replacement_retry_bind",
                        plan.transition_token,
                        plan.session_generation,
                        attempt,
                    );
                    continue;
                }
                Err(error) => {
                    self.record_error_if_current(plan.transition_token, &error)?;
                    return Err(error);
                }
            };
            let (task, shutdown_sender) = spawn_transport(&plan.transport, &mut prepared);
            let probed = probe_replacement_transport_async(
                prepared.plan.clone(),
                prepared.native_session_secret.clone(),
                self.webview_process_install_id.clone(),
                started_shell(),
                plan.probe_cancel.clone(),
            )
            .await;
            let probed = match probed {
                Ok(probed) => probed,
                Err(failure) => {
                    let candidate_exited = backend_requires_restart(Some(&task));
                    self.retire_uncommitted_candidate(task, shutdown_sender, plan.transition_token)
                        .await?;
                    if attempt < REPLACEMENT_ADMISSION_MAX_ATTEMPTS
                        && failure.is_retryable_startup()
                    {
                        replacement_checkpoint(
                            if candidate_exited {
                                "android_local_backend_replacement_retry_process_exit"
                            } else {
                                "android_local_backend_replacement_retry_probe"
                            },
                            plan.transition_token,
                            plan.session_generation,
                            attempt,
                        );
                        continue;
                    }
                    let error = failure.into_error();
                    self.record_error_if_current(plan.transition_token, &error)?;
                    return Err(error);
                }
            };

            let resumed = self
                .commit_replacement_candidate(&plan, prepared, probed, task, shutdown_sender)
                .await?;
            replacement_checkpoint(
                "android_local_backend_replacement_ready",
                plan.transition_token,
                plan.session_generation,
                attempt,
            );
            return Ok(resumed);
        }

        unreachable!("replacement admission attempt loop is non-empty")
    }

    async fn commit_replacement_candidate(
        &self,
        plan: &ResumePlan,
        prepared: super::super::generation::PreparedTransport,
        probed: super::super::generation::ProbedTransport,
        task: super::super::generation::BackendTask,
        shutdown_sender: tokio::sync::oneshot::Sender<()>,
    ) -> Result<MobileEmbeddedBackendResume, MobileEmbeddedBackendError> {
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
                self.retire_uncommitted_candidate(
                    candidate_task.expect("failed commit retains candidate task"),
                    candidate_shutdown.expect("failed commit retains candidate shutdown sender"),
                    plan.transition_token,
                )
                .await?;
                Err(error)
            }
        }
    }

    async fn retire_uncommitted_candidate(
        &self,
        task: super::super::generation::BackendTask,
        shutdown_sender: tokio::sync::oneshot::Sender<()>,
        transition_token: u64,
    ) -> Result<(), MobileEmbeddedBackendError> {
        match stop_transport(task, shutdown_sender).await {
            Ok(()) | Err(MobileEmbeddedBackendError::BackendExitedAfterSessionRetirement(_)) => {
                Ok(())
            }
            Err(error) => {
                self.record_retirement_failure(transition_token, &error)?;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_admission_retry_budget_is_one_fresh_attempt() {
        assert_eq!(REPLACEMENT_ADMISSION_MAX_ATTEMPTS, 2);
        assert!(retryable_prepare_failure(
            &MobileEmbeddedBackendError::PortAllocationFailed(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "test",
            ))
        ));
        assert!(!retryable_prepare_failure(
            &MobileEmbeddedBackendError::AuthMaterialGenerationFailed
        ));
    }

    #[test]
    fn replacement_checkpoints_are_fixed_and_secret_free() {
        let checkpoint = replacement_checkpoint_message(
            "android_local_backend_replacement_retry_probe",
            3,
            2,
            1,
        );
        assert_eq!(
            checkpoint,
            "deve_mobile LocalBackend replacement checkpoint: android_local_backend_replacement_retry_probe transition=3 generation=2 attempt=1"
        );
        assert!(!checkpoint.contains("secret"));
        assert!(!checkpoint.contains("cookie"));
        assert!(!checkpoint.contains("127.0.0.1"));
    }
}
