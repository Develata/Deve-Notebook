//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision
//!   - 11_ui_design/03_mobile#mobile-process-adapter-decision

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    NativeAdapterError, NativeEndpointReady, NativeServiceHealthProbe,
    validate_native_endpoint_bases, validate_native_endpoint_ready,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeProcessAdapterDecision {
    DeferredUntilPackagingGate,
    ExplicitNativeAuthorityOptIn,
    LocalBackendDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessAdapterPolicy {
    pub decision: NativeProcessAdapterDecision,
    pub child_process_runtime_enabled: bool,
    pub embedded_service_runtime_enabled: bool,
    pub packaging_gate_required: bool,
    pub authority_writes_allowed: bool,
}

impl NativeProcessAdapterPolicy {
    pub fn is_deferred_no_runtime(self) -> bool {
        self.decision == NativeProcessAdapterDecision::DeferredUntilPackagingGate
            && !self.child_process_runtime_enabled
            && !self.embedded_service_runtime_enabled
            && self.packaging_gate_required
            && !self.authority_writes_allowed
    }

    pub fn is_explicit_desktop_native_authority_opt_in(self) -> bool {
        self.decision == NativeProcessAdapterDecision::ExplicitNativeAuthorityOptIn
            && self.child_process_runtime_enabled
            && !self.embedded_service_runtime_enabled
            && self.packaging_gate_required
            && self.authority_writes_allowed
    }

    pub fn is_explicit_mobile_native_authority_opt_in(self) -> bool {
        self.decision == NativeProcessAdapterDecision::ExplicitNativeAuthorityOptIn
            && !self.child_process_runtime_enabled
            && self.embedded_service_runtime_enabled
            && self.packaging_gate_required
            && self.authority_writes_allowed
    }

    pub fn is_desktop_local_backend_default(self) -> bool {
        self.decision == NativeProcessAdapterDecision::LocalBackendDefault
            && self.child_process_runtime_enabled
            && !self.embedded_service_runtime_enabled
            && self.packaging_gate_required
            && !self.authority_writes_allowed
    }

    pub fn is_mobile_local_backend_default(self) -> bool {
        self.decision == NativeProcessAdapterDecision::LocalBackendDefault
            && !self.child_process_runtime_enabled
            && self.embedded_service_runtime_enabled
            && self.packaging_gate_required
            && !self.authority_writes_allowed
    }
}

pub const CURRENT_NATIVE_PROCESS_ADAPTER_POLICY: NativeProcessAdapterPolicy =
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
        child_process_runtime_enabled: false,
        embedded_service_runtime_enabled: false,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    };

pub fn desktop_native_authority_policy_from_env() -> NativeProcessAdapterPolicy {
    if env_flag("DEVE_NATIVE_AUTHORITY") && env_flag("DEVE_DESKTOP_LOCAL_SERVICE") {
        NativeProcessAdapterPolicy {
            decision: NativeProcessAdapterDecision::ExplicitNativeAuthorityOptIn,
            child_process_runtime_enabled: true,
            embedded_service_runtime_enabled: false,
            packaging_gate_required: true,
            authority_writes_allowed: true,
        }
    } else {
        CURRENT_NATIVE_PROCESS_ADAPTER_POLICY
    }
}

pub fn mobile_native_authority_policy_from_env() -> NativeProcessAdapterPolicy {
    if env_flag("DEVE_NATIVE_AUTHORITY") && env_flag("DEVE_MOBILE_EMBEDDED_SERVICE") {
        NativeProcessAdapterPolicy {
            decision: NativeProcessAdapterDecision::ExplicitNativeAuthorityOptIn,
            child_process_runtime_enabled: false,
            embedded_service_runtime_enabled: true,
            packaging_gate_required: true,
            authority_writes_allowed: true,
        }
    } else {
        CURRENT_NATIVE_PROCESS_ADAPTER_POLICY
    }
}

pub fn desktop_local_backend_policy() -> NativeProcessAdapterPolicy {
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::LocalBackendDefault,
        child_process_runtime_enabled: true,
        embedded_service_runtime_enabled: false,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    }
}

pub fn mobile_local_backend_policy() -> NativeProcessAdapterPolicy {
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::LocalBackendDefault,
        child_process_runtime_enabled: false,
        embedded_service_runtime_enabled: true,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeProcessAdapterState {
    Deferred,
    ExistingEndpointBound,
    SessionHandoffReady,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessAdapterSnapshot {
    pub state: NativeProcessAdapterState,
    pub endpoint: Option<NativeEndpointReady>,
    pub health_probe: NativeServiceHealthProbe,
    pub child_process_runtime_enabled: bool,
    pub embedded_service_runtime_enabled: bool,
    pub child_process_running: bool,
    pub authority_writes_allowed: bool,
}

impl NativeProcessAdapterSnapshot {
    pub fn is_default_safe_boundary(&self) -> bool {
        !self.child_process_runtime_enabled
            && !self.embedded_service_runtime_enabled
            && !self.child_process_running
            && !self.authority_writes_allowed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NativeProcessAdapterError {
    #[error("native child-process runtime is disabled by process adapter policy")]
    ChildProcessRuntimeDisabled,
    #[error("native process adapter endpoint is invalid: {0}")]
    InvalidEndpoint(#[from] NativeAdapterError),
    #[error("native process adapter endpoint is not bound")]
    EndpointNotBound,
    #[error("native process adapter session is not bound")]
    SessionNotBound,
}

#[derive(Debug, Clone)]
pub struct NativeProcessAdapter {
    policy: NativeProcessAdapterPolicy,
    state: NativeProcessAdapterState,
    endpoint: Option<NativeEndpointReady>,
    health_probe: NativeServiceHealthProbe,
    child_process_running: bool,
}

impl Default for NativeProcessAdapter {
    fn default() -> Self {
        Self::new(CURRENT_NATIVE_PROCESS_ADAPTER_POLICY)
    }
}

impl NativeProcessAdapter {
    pub fn new(policy: NativeProcessAdapterPolicy) -> Self {
        Self {
            policy,
            state: NativeProcessAdapterState::Deferred,
            endpoint: None,
            health_probe: NativeServiceHealthProbe::default(),
            child_process_running: false,
        }
    }

    pub fn ensure_child_process_runtime_enabled(&self) -> Result<(), NativeProcessAdapterError> {
        if self.policy.child_process_runtime_enabled {
            Ok(())
        } else {
            Err(NativeProcessAdapterError::ChildProcessRuntimeDisabled)
        }
    }

    pub fn bind_existing_endpoint(
        &mut self,
        mut endpoint: NativeEndpointReady,
    ) -> Result<NativeProcessAdapterSnapshot, NativeProcessAdapterError> {
        endpoint.session_bound = false;
        validate_native_endpoint_bases(&endpoint)?;

        self.health_probe = NativeServiceHealthProbe {
            endpoint_reachable: true,
            node_role_readable: true,
        };
        self.endpoint = Some(endpoint);
        self.state = NativeProcessAdapterState::ExistingEndpointBound;
        Ok(self.snapshot())
    }

    pub fn bind_session(
        &mut self,
        session_bound: bool,
    ) -> Result<NativeProcessAdapterSnapshot, NativeProcessAdapterError> {
        if !session_bound {
            return Err(NativeProcessAdapterError::SessionNotBound);
        }
        let endpoint = self
            .endpoint
            .as_mut()
            .ok_or(NativeProcessAdapterError::EndpointNotBound)?;
        endpoint.session_bound = true;
        validate_native_endpoint_ready(endpoint)?;
        self.state = NativeProcessAdapterState::SessionHandoffReady;
        Ok(self.snapshot())
    }

    pub fn record_health_probe(
        &mut self,
        probe: NativeServiceHealthProbe,
    ) -> NativeProcessAdapterSnapshot {
        self.health_probe = probe;
        self.snapshot()
    }

    pub fn record_probe_timeout(&mut self) -> NativeProcessAdapterSnapshot {
        self.record_health_probe(NativeServiceHealthProbe::default())
    }

    pub fn clear_session(&mut self) {
        if let Some(endpoint) = self.endpoint.as_mut() {
            endpoint.session_bound = false;
        }
        if self.state == NativeProcessAdapterState::SessionHandoffReady {
            self.state = NativeProcessAdapterState::ExistingEndpointBound;
        }
    }

    pub fn record_process_stopped(&mut self) -> NativeProcessAdapterSnapshot {
        self.state = NativeProcessAdapterState::Stopped;
        self.endpoint = None;
        self.health_probe = NativeServiceHealthProbe::default();
        self.child_process_running = false;
        self.snapshot()
    }

    pub fn snapshot(&self) -> NativeProcessAdapterSnapshot {
        NativeProcessAdapterSnapshot {
            state: self.state,
            endpoint: self.endpoint.clone(),
            health_probe: self.health_probe,
            child_process_runtime_enabled: self.policy.child_process_runtime_enabled,
            embedded_service_runtime_enabled: self.policy.embedded_service_runtime_enabled,
            child_process_running: self.child_process_running,
            authority_writes_allowed: self.policy.authority_writes_allowed,
        }
    }
}
