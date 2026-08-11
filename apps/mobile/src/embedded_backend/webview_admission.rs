//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!
//! One-time native lifecycle admission for recovery-created WebView handoffs.

#[cfg(any(mobile, test))]
use std::time::Duration;

use tokio::sync::watch;

use super::MobileEmbeddedBackendError;

#[cfg(any(mobile, test))]
const INITIAL_WEBVIEW_SESSION_ADMISSION_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdmissionState {
    Open,
    Deferred,
    Granted,
    #[cfg(any(mobile, test))]
    TimedOut,
    Cancelled,
}

pub(super) struct InitialWebviewSessionAdmission {
    state: watch::Sender<AdmissionState>,
}

impl InitialWebviewSessionAdmission {
    pub(super) fn new() -> Self {
        let (state, _) = watch::channel(AdmissionState::Open);
        Self { state }
    }

    pub(super) fn defer_for_recovery(&self) -> Result<(), MobileEmbeddedBackendError> {
        self.state
            .send_if_modified(|state| {
                if *state != AdmissionState::Open {
                    return false;
                }
                *state = AdmissionState::Deferred;
                true
            })
            .then_some(())
            .ok_or(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionInvalid)
    }

    pub(super) fn admit_recovery(&self) -> Result<(), MobileEmbeddedBackendError> {
        self.state
            .send_if_modified(|state| {
                if *state != AdmissionState::Deferred {
                    return false;
                }
                *state = AdmissionState::Granted;
                true
            })
            .then_some(())
            .ok_or(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionInvalid)
    }

    pub(super) fn cancel(&self) {
        self.state.send_if_modified(|state| {
            if *state == AdmissionState::Cancelled {
                return false;
            }
            #[cfg(any(mobile, test))]
            if *state == AdmissionState::TimedOut {
                return false;
            }
            *state = AdmissionState::Cancelled;
            true
        });
    }

    #[cfg(any(mobile, test))]
    pub(super) async fn wait(&self) -> Result<(), MobileEmbeddedBackendError> {
        self.wait_with_limit(INITIAL_WEBVIEW_SESSION_ADMISSION_TIMEOUT)
            .await
    }

    #[cfg(mobile)]
    pub(super) fn ensure_handoff_allowed(&self) -> Result<(), MobileEmbeddedBackendError> {
        match *self.state.borrow() {
            AdmissionState::Open | AdmissionState::Granted => Ok(()),
            AdmissionState::TimedOut => {
                Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionTimeout)
            }
            AdmissionState::Cancelled => {
                Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionCancelled)
            }
            AdmissionState::Deferred => {
                Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionInvalid)
            }
        }
    }

    #[cfg(any(mobile, test))]
    async fn wait_with_limit(&self, limit: Duration) -> Result<(), MobileEmbeddedBackendError> {
        let deadline = tokio::time::Instant::now() + limit;
        let mut state = self.state.subscribe();
        loop {
            match *state.borrow_and_update() {
                AdmissionState::Open | AdmissionState::Granted => return Ok(()),
                AdmissionState::TimedOut => {
                    return Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionTimeout);
                }
                AdmissionState::Cancelled => {
                    return Err(
                        MobileEmbeddedBackendError::InitialWebviewSessionAdmissionCancelled,
                    );
                }
                AdmissionState::Deferred => {}
            }
            match tokio::time::timeout_at(deadline, state.changed()).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    return Err(
                        MobileEmbeddedBackendError::InitialWebviewSessionAdmissionCancelled,
                    );
                }
                Err(_) => {
                    self.state.send_if_modified(|state| {
                        if *state != AdmissionState::Deferred {
                            return false;
                        }
                        *state = AdmissionState::TimedOut;
                        true
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn cold_local_backend_webview_handoff_is_immediately_admitted() {
        InitialWebviewSessionAdmission::new()
            .wait_with_limit(Duration::from_millis(20))
            .await
            .expect("cold LocalBackend admission");
    }

    #[test]
    fn recovery_admission_budget_exceeds_platform_ack_and_anchor_retirement_bounds() {
        assert!(INITIAL_WEBVIEW_SESSION_ADMISSION_TIMEOUT > Duration::from_secs(5 + 10));
    }

    #[tokio::test]
    async fn android_recovery_webview_handoff_waits_for_native_surface_admission() {
        let admission = Arc::new(InitialWebviewSessionAdmission::new());
        admission.defer_for_recovery().expect("defer recovery");
        let waiting = admission.clone();
        let mut waiter = tokio::spawn(async move { waiting.wait().await });

        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut waiter)
                .await
                .is_err(),
            "recovery WebView handoff bypassed native surface admission"
        );
        admission.admit_recovery().expect("admit recovery");
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("admission wake timeout")
            .expect("admission task")
            .expect("admission result");
    }

    #[tokio::test]
    async fn timed_out_recovery_admission_rejects_late_grant() {
        let admission = InitialWebviewSessionAdmission::new();
        admission.defer_for_recovery().expect("defer recovery");
        assert!(matches!(
            admission.wait_with_limit(Duration::from_millis(20)).await,
            Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionTimeout)
        ));
        assert!(matches!(
            admission.admit_recovery(),
            Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionInvalid)
        ));
    }

    #[tokio::test]
    async fn cancellation_wakes_all_deferred_handoffs_fail_closed() {
        let admission = Arc::new(InitialWebviewSessionAdmission::new());
        admission.defer_for_recovery().expect("defer recovery");
        let first = {
            let admission = admission.clone();
            tokio::spawn(async move { admission.wait().await })
        };
        let second = {
            let admission = admission.clone();
            tokio::spawn(async move { admission.wait().await })
        };
        tokio::task::yield_now().await;
        admission.cancel();
        for waiter in [first, second] {
            assert!(matches!(
                waiter.await.expect("admission waiter"),
                Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionCancelled)
            ));
        }
    }

    #[test]
    fn recovery_admission_rejects_invalid_or_repeated_transitions() {
        let admission = InitialWebviewSessionAdmission::new();
        assert!(matches!(
            admission.admit_recovery(),
            Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionInvalid)
        ));
        admission.defer_for_recovery().expect("defer recovery");
        assert!(matches!(
            admission.defer_for_recovery(),
            Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionInvalid)
        ));
        admission.admit_recovery().expect("admit recovery");
        assert!(matches!(
            admission.admit_recovery(),
            Err(MobileEmbeddedBackendError::InitialWebviewSessionAdmissionInvalid)
        ));
    }
}
