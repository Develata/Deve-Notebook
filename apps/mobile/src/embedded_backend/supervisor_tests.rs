use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{Notify, oneshot};

use super::*;
use crate::MobileShell;
use crate::embedded_backend::generation::{
    BackendTask, BackendTaskFailure, await_transport_task, backend_requires_restart,
};
use crate::embedded_backend::supervisor_types::ensure_current_resume;
use crate::embedded_backend::{MobileEmbeddedBackendError, plan_mobile_embedded_backend};

fn test_generation(task: Option<BackendTask>) -> BackendGeneration {
    let root = std::env::current_dir()
        .expect("cwd")
        .join("target/mobile-test-data");
    BackendGeneration {
        runtime: None,
        plan: plan_mobile_embedded_backend(root, 40123).expect("plan"),
        native_session_cookie:
            crate::embedded_backend::cookie::MobileNativeSessionCookie::from_set_cookie(
                "token=test; Path=/; HttpOnly; SameSite=None; Secure",
                "127.0.0.1",
            )
            .expect("test cookie"),
        task,
        shutdown_sender: None,
        transport_stopping: false,
        runtime_restart_required: false,
        probe_cancel: None,
        shell: MobileShell::new(),
        session_generation: 7,
        transition_token: 3,
        service_state: MobileEmbeddedBackendServiceState::EndpointSessionReady,
        last_error: None,
        last_error_transition_token: None,
    }
}

fn test_supervisor(generation: BackendGeneration) -> MobileEmbeddedBackendSupervisor {
    MobileEmbeddedBackendSupervisor {
        app_data_dir: generation.plan.app_data_dir.clone(),
        webview_process_install_id: "test-process-session".to_string(),
        inner: Mutex::new(generation),
        active_resumes: AtomicUsize::new(0),
        resumes_idle: Notify::new(),
        webview_handoff_gate: tokio::sync::Mutex::new(()),
        initial_webview_session_admission:
            crate::embedded_backend::webview_admission::InitialWebviewSessionAdmission::new(),
    }
}

struct RuntimeTestRoot {
    path: std::path::PathBuf,
    allowed_parent: std::path::PathBuf,
}

impl RuntimeTestRoot {
    fn new() -> Self {
        let allowed_parent = std::env::current_dir()
            .expect("cwd")
            .join("target/mobile-test-data/runtime-replacement");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = allowed_parent.join(unique.to_string());
        Self {
            path,
            allowed_parent,
        }
    }
}

impl Drop for RuntimeTestRoot {
    fn drop(&mut self) {
        assert!(self.path.starts_with(&self.allowed_parent));
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[tokio::test]
async fn android_initial_and_resume_webview_session_handoffs_share_process_single_flight_gate() {
    let supervisor = std::sync::Arc::new(test_supervisor(test_generation(None)));
    let first_handoff = supervisor.webview_handoff_gate.lock().await;
    let (entered_sender, mut entered_receiver) = tokio::sync::mpsc::channel(1);
    let concurrent = supervisor.clone();
    let waiter = tokio::spawn(async move {
        let _handoff = concurrent.webview_handoff_gate.lock().await;
        entered_sender.send(()).await.expect("handoff admitted");
    });

    assert!(
        tokio::time::timeout(Duration::from_millis(20), entered_receiver.recv())
            .await
            .is_err(),
        "a concurrent WebView handoff bypassed the process-local gate"
    );
    drop(first_handoff);
    tokio::time::timeout(Duration::from_secs(1), entered_receiver.recv())
        .await
        .expect("handoff admission timeout")
        .expect("handoff channel closed");
    waiter.await.expect("handoff waiter");
}

#[test]
fn android_initial_and_resume_handoff_production_wiring_uses_shared_gate() {
    let source = include_str!("supervisor_webview.rs");
    assert_eq!(
        source
            .matches("let _handoff_gate = self.webview_handoff_gate.lock().await;")
            .count(),
        2,
        "initial prepare and resume must both acquire the shared WebView handoff gate"
    );
    assert_eq!(
        source
            .matches("self.initial_webview_session_admission.wait().await?;")
            .count(),
        2,
        "initial prepare and resume must both await native surface admission"
    );
    assert_eq!(
        source.matches(".ensure_handoff_allowed()?").count(),
        2,
        "initial prepare and resume must recheck admission inside the handoff gate"
    );
}

#[test]
fn mobile_embedded_backend_supervisor_snapshot_tracks_owned_generation() {
    let generation = test_generation(None);
    let snapshot = snapshot_from_generation(&generation);

    assert_eq!(snapshot.endpoint.as_deref(), Some("http://127.0.0.1:40123"));
    assert_eq!(snapshot.session_generation, 7);
    assert!(!snapshot.backend_running);
    assert_eq!(
        snapshot.service_state,
        MobileEmbeddedBackendServiceState::EndpointSessionReady
    );
}

#[tokio::test]
async fn mobile_embedded_backend_supervisor_shutdown_is_bounded_and_owned() {
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let task = tauri::async_runtime::spawn(async move {
        let _ = shutdown_receiver.await;
        Ok(())
    });
    let mut generation = test_generation(Some(task));
    generation.shutdown_sender = Some(shutdown_sender);
    let probe_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    generation.probe_cancel = Some(probe_cancel.clone());
    let supervisor = test_supervisor(generation);

    supervisor
        .shutdown(Duration::from_secs(1))
        .await
        .expect("shutdown");
    assert!(matches!(
        supervisor.initial_webview_session_admission.wait().await,
        Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionCancelled)
    ));
    assert!(probe_cancel.load(Ordering::Acquire));
    let snapshot = supervisor.snapshot().expect("snapshot");
    assert_eq!(
        snapshot.service_state,
        MobileEmbeddedBackendServiceState::Stopped
    );
    assert!(!snapshot.backend_running);
}

#[tokio::test]
async fn mobile_embedded_backend_supervisor_shutdown_drains_webview_handoff_gate() {
    let supervisor = Arc::new(test_supervisor(test_generation(None)));
    let handoff = supervisor.webview_handoff_gate.lock().await;
    let stopping = supervisor.clone();
    let mut shutdown = tokio::spawn(async move { stopping.shutdown(Duration::from_secs(1)).await });

    tokio::task::yield_now().await;
    assert!(matches!(
        supervisor.initial_webview_session_admission.wait().await,
        Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionCancelled)
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut shutdown)
            .await
            .is_err(),
        "shutdown completed before the in-flight WebView handoff gate drained"
    );
    drop(handoff);
    tokio::time::timeout(Duration::from_secs(1), shutdown)
        .await
        .expect("shutdown drain timeout")
        .expect("shutdown task")
        .expect("shutdown result");
}

#[test]
fn mobile_embedded_backend_supervisor_missing_or_finished_task_requires_restart() {
    assert!(backend_requires_restart(None));
    let task = tauri::async_runtime::spawn(async { Ok(()) });
    tauri::async_runtime::block_on(async {
        while !task.inner().is_finished() {
            tokio::task::yield_now().await;
        }
    });
    assert!(backend_requires_restart(Some(&task)));
}

#[test]
fn lifecycle_fault_injection_marks_transport_stopping_before_task_exit() {
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let task = tauri::async_runtime::spawn(async move {
        let _ = shutdown_receiver.await;
        Ok(())
    });
    let mut generation = test_generation(Some(task));
    generation.shutdown_sender = Some(shutdown_sender);
    let supervisor = test_supervisor(generation);

    supervisor
        .stop_transport_for_lifecycle_smoke()
        .expect("fault injection");

    let snapshot = supervisor.snapshot().expect("snapshot");
    assert!(!snapshot.backend_running);
}

#[test]
fn mobile_resume_restarts_dead_backend_on_new_random_endpoint() {
    let root = RuntimeTestRoot::new();
    let (supervisor, _) = MobileEmbeddedBackendSupervisor::start(root.path.clone())
        .expect("start embedded backend supervisor");
    let initial = supervisor.snapshot().expect("initial snapshot");

    supervisor.suspend().expect("suspend");
    supervisor
        .stop_transport_for_lifecycle_smoke()
        .expect("stop current transport");
    let resumed = tauri::async_runtime::block_on(supervisor.resume_transition())
        .expect("resume with replacement transport");
    let replacement = supervisor.snapshot().expect("replacement snapshot");

    assert!(resumed.restarted);
    assert_eq!(
        replacement.session_generation,
        initial.session_generation + 1
    );
    assert_ne!(replacement.endpoint, initial.endpoint);
    assert!(replacement.backend_running);
    assert_eq!(
        replacement.service_state,
        MobileEmbeddedBackendServiceState::EndpointSessionReady
    );

    tauri::async_runtime::block_on(supervisor.shutdown(Duration::from_secs(10)))
        .expect("shutdown replacement runtime");
}

#[test]
fn failed_transport_retirement_permanently_requires_app_restart() {
    let mut generation = test_generation(None);
    generation.runtime_restart_required = true;
    generation.service_state = MobileEmbeddedBackendServiceState::Error;
    let supervisor = test_supervisor(generation);

    assert!(matches!(
        supervisor.begin_resume(),
        Err(MobileEmbeddedBackendError::RuntimeRestartRequired)
    ));
}

#[test]
fn retirement_failure_cannot_be_discarded_by_newer_lifecycle_transition() {
    let mut generation = test_generation(None);
    generation.transition_token = 99;
    generation.service_state = MobileEmbeddedBackendServiceState::BackgroundSuspended;
    let supervisor = test_supervisor(generation);

    supervisor
        .record_retirement_failure(3, &MobileEmbeddedBackendError::ShutdownTimeout)
        .expect("record retirement failure");

    assert!(
        supervisor
            .lock_inner()
            .expect("inner")
            .runtime_restart_required
    );
}

#[tokio::test]
async fn transport_exit_distinguishes_clean_session_retirement_from_unsafe_failure() {
    let retired = tauri::async_runtime::spawn(async {
        Err(BackendTaskFailure {
            message: "listener failed".to_string(),
            sessions_retired: true,
        })
    });
    let unsafe_exit = tauri::async_runtime::spawn(async {
        Err(BackendTaskFailure {
            message: "session retirement failed".to_string(),
            sessions_retired: false,
        })
    });

    assert!(matches!(
        await_transport_task(
            retired,
            tokio::time::Instant::now() + Duration::from_secs(1)
        )
        .await,
        Err(MobileEmbeddedBackendError::BackendExitedAfterSessionRetirement(_))
    ));
    assert!(matches!(
        await_transport_task(
            unsafe_exit,
            tokio::time::Instant::now() + Duration::from_secs(1)
        )
        .await,
        Err(MobileEmbeddedBackendError::BackendExited(_))
    ));
}

#[test]
fn transient_resume_error_forces_fresh_transport_and_shell() {
    let supervisor = test_supervisor(test_generation(None));

    supervisor
        .record_error_if_current(3, &MobileEmbeddedBackendError::ProbeInvalidResponse)
        .expect("record error");

    let inner = supervisor.lock_inner().expect("inner");
    assert!(inner.transport_stopping);
    assert!(!inner.runtime_restart_required);
}

#[test]
fn stale_resume_transition_is_rejected_without_state_mutation() {
    let generation = test_generation(None);
    assert!(matches!(
        ensure_current_transition(&generation, 2),
        Err(MobileEmbeddedBackendError::LifecycleTransitionCancelled)
    ));
    assert!(ensure_current_transition(&generation, 3).is_ok());
}

#[test]
#[cfg(not(mobile))]
fn stale_resume_install_cannot_replace_current_generation() {
    let generation = test_generation(None);
    let stale = MobileEmbeddedBackendResume {
        restarted: true,
        session_generation: 6,
        transition_token: 2,
    };
    let current = MobileEmbeddedBackendResume {
        restarted: false,
        session_generation: 7,
        transition_token: 3,
    };

    assert!(matches!(
        ensure_current_resume(&generation, &stale),
        Err(MobileEmbeddedBackendError::LifecycleTransitionCancelled)
    ));
    assert!(ensure_current_resume(&generation, &current).is_ok());
}
