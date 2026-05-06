//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-service-supervisor-contract
//!   - 08_ui_design_03_mobile#mobile-service-supervisor-contract

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{NativeProcessAdapterSnapshot, NativeProcessAdapterState, NativeServiceOffline};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeServiceSupervisorState {
    Idle,
    Starting,
    EndpointHealthy,
    SessionHandoffReady,
    Restarting,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeServiceFailureKind {
    SpawnFailed,
    BindFailed,
    HealthProbeFailed,
    ProcessExited,
    SessionHandoffFailed,
}

impl NativeServiceFailureKind {
    pub fn retryable_by_default(self) -> bool {
        matches!(
            self,
            Self::BindFailed | Self::HealthProbeFailed | Self::ProcessExited
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeServiceHealthProbe {
    pub endpoint_reachable: bool,
    pub node_role_readable: bool,
}

impl NativeServiceHealthProbe {
    pub fn is_healthy(self) -> bool {
        self.endpoint_reachable && self.node_role_readable
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeServiceSupervisorSnapshot {
    pub state: NativeServiceSupervisorState,
    pub restart_attempt: u32,
    pub max_restart_attempts: u32,
    pub offline: Option<NativeServiceOffline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeServiceSupervisorObservation {
    Idle,
    EndpointHealthy,
    SessionHandoffReady,
    Offline(NativeServiceOffline),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NativeServiceSupervisorError {
    #[error("native service health probe failed")]
    HealthProbeFailed,
    #[error("native service session handoff requires a healthy endpoint")]
    EndpointNotHealthy,
    #[error("native service session is not bound")]
    SessionNotBound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeServiceSupervisor {
    state: NativeServiceSupervisorState,
    restart_attempt: u32,
    max_restart_attempts: u32,
    offline: Option<NativeServiceOffline>,
}

impl NativeServiceSupervisor {
    pub fn new(max_restart_attempts: u32) -> Self {
        Self {
            state: NativeServiceSupervisorState::Idle,
            restart_attempt: 0,
            max_restart_attempts,
            offline: None,
        }
    }

    pub fn start(&mut self) {
        if self.terminal_offline().is_some() {
            return;
        }
        self.state = NativeServiceSupervisorState::Starting;
        self.offline = None;
    }

    pub fn record_process_snapshot(
        &mut self,
        snapshot: &NativeProcessAdapterSnapshot,
    ) -> NativeServiceSupervisorObservation {
        if let Some(current) = self.terminal_offline() {
            return NativeServiceSupervisorObservation::Offline(current);
        }

        match snapshot.state {
            NativeProcessAdapterState::Deferred => NativeServiceSupervisorObservation::Idle,
            NativeProcessAdapterState::ExistingEndpointBound => {
                self.record_endpoint_health_snapshot(snapshot)
            }
            NativeProcessAdapterState::SessionHandoffReady => {
                self.record_session_snapshot(snapshot)
            }
            NativeProcessAdapterState::Stopped => NativeServiceSupervisorObservation::Offline(
                self.record_failure(NativeServiceFailureKind::ProcessExited, "process_stopped"),
            ),
        }
    }

    pub fn record_failure(
        &mut self,
        kind: NativeServiceFailureKind,
        reason: impl Into<String>,
    ) -> NativeServiceOffline {
        if let Some(current) = self.terminal_offline() {
            return current;
        }
        let retryable =
            kind.retryable_by_default() && self.restart_attempt < self.max_restart_attempts;
        if retryable {
            self.restart_attempt += 1;
            self.state = NativeServiceSupervisorState::Restarting;
        } else {
            self.state = NativeServiceSupervisorState::Offline;
        }
        let offline = NativeServiceOffline {
            reason: reason.into(),
            retryable,
        };
        self.offline = Some(offline.clone());
        offline
    }

    pub fn record_service_offline(
        &mut self,
        offline: NativeServiceOffline,
    ) -> NativeServiceOffline {
        if let Some(current) = self.terminal_offline() {
            return current;
        }
        let retryable = offline.retryable && self.restart_attempt < self.max_restart_attempts;
        let recorded = NativeServiceOffline {
            reason: offline.reason,
            retryable,
        };
        self.state = if recorded.retryable {
            NativeServiceSupervisorState::Restarting
        } else {
            NativeServiceSupervisorState::Offline
        };
        self.offline = Some(recorded.clone());
        recorded
    }

    pub fn snapshot(&self) -> NativeServiceSupervisorSnapshot {
        NativeServiceSupervisorSnapshot {
            state: self.state,
            restart_attempt: self.restart_attempt,
            max_restart_attempts: self.max_restart_attempts,
            offline: self.offline.clone(),
        }
    }

    fn terminal_offline(&self) -> Option<NativeServiceOffline> {
        if self.state != NativeServiceSupervisorState::Offline {
            return None;
        }
        self.offline
            .as_ref()
            .filter(|offline| !offline.retryable)
            .cloned()
    }

    fn record_endpoint_health_snapshot(
        &mut self,
        snapshot: &NativeProcessAdapterSnapshot,
    ) -> NativeServiceSupervisorObservation {
        if !snapshot.health_probe.is_healthy() || snapshot.endpoint.is_none() {
            return NativeServiceSupervisorObservation::Offline(
                self.record_failure(NativeServiceFailureKind::HealthProbeFailed, "probe_failed"),
            );
        }

        self.state = NativeServiceSupervisorState::EndpointHealthy;
        self.offline = None;
        NativeServiceSupervisorObservation::EndpointHealthy
    }

    fn record_session_snapshot(
        &mut self,
        snapshot: &NativeProcessAdapterSnapshot,
    ) -> NativeServiceSupervisorObservation {
        if !snapshot.health_probe.is_healthy() {
            return NativeServiceSupervisorObservation::Offline(
                self.record_failure(NativeServiceFailureKind::HealthProbeFailed, "probe_failed"),
            );
        }
        let session_bound = snapshot
            .endpoint
            .as_ref()
            .map(|endpoint| endpoint.session_bound)
            .unwrap_or(false);
        if !session_bound {
            return NativeServiceSupervisorObservation::Offline(self.record_failure(
                NativeServiceFailureKind::SessionHandoffFailed,
                "session_not_bound",
            ));
        }

        self.state = NativeServiceSupervisorState::SessionHandoffReady;
        self.offline = None;
        NativeServiceSupervisorObservation::SessionHandoffReady
    }
}
