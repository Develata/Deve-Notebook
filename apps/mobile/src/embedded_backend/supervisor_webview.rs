//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!
//! Current-generation WebView cookie/bootstrap/event handoff.

use super::super::cookie::install_native_session_cookie_confirmed;
use super::super::supervisor_types::ensure_current_resume;
use super::super::{
    MobileEmbeddedBackendError, MobileEmbeddedBackendResume, supervisor_types::record_error,
};
use super::MobileEmbeddedBackendSupervisor;

impl MobileEmbeddedBackendSupervisor {
    pub(crate) async fn prepare_initial_webview_session<R: tauri::Runtime>(
        &self,
        webview: &tauri::WebviewWindow<R>,
    ) -> Result<(), MobileEmbeddedBackendError> {
        let cookie = self.lock_inner()?.native_session_cookie.clone();
        install_native_session_cookie_confirmed(webview, &cookie)
            .await
            .map_err(MobileEmbeddedBackendError::WebviewInstallFailed)
    }

    pub(crate) async fn resume_and_complete_on_webview<R: tauri::Runtime>(
        &self,
        webview: &tauri::WebviewWindow<R>,
        resumed_event: &str,
        error_event: &str,
    ) -> Result<(), MobileEmbeddedBackendError> {
        let _resume_gate = self.resume_gate.lock().await;
        let _activity = super::ResumeActivity::new(self);
        let result = self
            .resume_and_install_current(webview, resumed_event)
            .await;
        if let Err(error) = &result
            && !matches!(
                error,
                MobileEmbeddedBackendError::LifecycleTransitionCancelled
            )
        {
            let transition_token = self
                .lock_inner()
                .ok()
                .and_then(|inner| inner.last_error_transition_token);
            if let Some(transition_token) = transition_token
                && let Err(dispatch_error) = webview.eval(guarded_lifecycle_event_source(
                    transition_token,
                    error_event,
                ))
            {
                eprintln!(
                    "deve_mobile guarded lifecycle error dispatch failed closed: {dispatch_error}"
                );
            }
        }
        result
    }

    async fn resume_and_install_current<R: tauri::Runtime>(
        &self,
        webview: &tauri::WebviewWindow<R>,
        resumed_event: &str,
    ) -> Result<(), MobileEmbeddedBackendError> {
        let resumed = self.resume_transition().await?;

        {
            let inner = self.lock_inner()?;
            ensure_current_resume(&inner, &resumed)?;
        }
        if let Err(source) = resumed.install_on_webview(webview).await {
            return Err(self.record_handoff_error_if_current(
                &resumed,
                MobileEmbeddedBackendError::WebviewInstallFailed(source),
            ));
        }
        {
            let inner = self.lock_inner()?;
            ensure_current_resume(&inner, &resumed)?;
        }
        let event_source = guarded_lifecycle_event_source(resumed.transition_token, resumed_event);
        if let Err(source) = webview.eval(event_source) {
            return Err(self.record_handoff_error_if_current(
                &resumed,
                MobileEmbeddedBackendError::WebviewInstallFailed(source.to_string()),
            ));
        }
        let inner = self.lock_inner()?;
        ensure_current_resume(&inner, &resumed)
    }

    pub(crate) fn record_resume_webview_unavailable(
        &self,
    ) -> Result<(), MobileEmbeddedBackendError> {
        let mut inner = self.lock_inner()?;
        if matches!(
            inner.service_state,
            super::super::MobileEmbeddedBackendServiceState::Stopping
                | super::super::MobileEmbeddedBackendServiceState::Stopped
        ) {
            return Err(MobileEmbeddedBackendError::LifecycleTransitionCancelled);
        }
        let error = MobileEmbeddedBackendError::WebviewUnavailable;
        inner.transport_stopping = true;
        record_error(&mut inner, &error);
        inner.last_error_transition_token = Some(inner.transition_token);
        Err(error)
    }

    fn record_handoff_error_if_current(
        &self,
        resumed: &MobileEmbeddedBackendResume,
        error: MobileEmbeddedBackendError,
    ) -> MobileEmbeddedBackendError {
        let Ok(mut inner) = self.lock_inner() else {
            return MobileEmbeddedBackendError::SupervisorStatePoisoned;
        };
        if ensure_current_resume(&inner, resumed).is_err() {
            return MobileEmbeddedBackendError::LifecycleTransitionCancelled;
        }
        inner.transport_stopping = true;
        record_error(&mut inner, &error);
        inner.last_error_transition_token = Some(resumed.transition_token);
        error
    }
}

pub(super) fn guarded_lifecycle_event_source(transition_token: u64, event: &str) -> String {
    format!(
        "(()=>{{const k='__DEVE_NATIVE_LIFECYCLE_TRANSITION__';const n={transition_token};const c=Number(window[k]??0);if(c<n){{window[k]=n;window.dispatchEvent(new Event({event:?}));}}}})();"
    )
}
