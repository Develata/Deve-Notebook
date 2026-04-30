//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-service-supervisor-contract
//!   - 08_ui_design_03_mobile#mobile-service-supervisor-contract

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::NativeServiceOffline;

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
        self.state = NativeServiceSupervisorState::Starting;
        self.offline = None;
    }

    pub fn record_health_probe(
        &mut self,
        probe: NativeServiceHealthProbe,
    ) -> Result<(), NativeServiceSupervisorError> {
        if !probe.is_healthy() {
            return Err(NativeServiceSupervisorError::HealthProbeFailed);
        }
        self.state = NativeServiceSupervisorState::EndpointHealthy;
        self.offline = None;
        Ok(())
    }

    pub fn record_session_handoff(
        &mut self,
        session_bound: bool,
    ) -> Result<(), NativeServiceSupervisorError> {
        if self.state != NativeServiceSupervisorState::EndpointHealthy {
            return Err(NativeServiceSupervisorError::EndpointNotHealthy);
        }
        if !session_bound {
            return Err(NativeServiceSupervisorError::SessionNotBound);
        }
        self.state = NativeServiceSupervisorState::SessionHandoffReady;
        Ok(())
    }

    pub fn record_failure(
        &mut self,
        kind: NativeServiceFailureKind,
        reason: impl Into<String>,
    ) -> NativeServiceOffline {
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
        let retryable = offline.retryable && self.restart_attempt < self.max_restart_attempts;
        if retryable {
            self.restart_attempt += 1;
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service_probe() -> NativeServiceHealthProbe {
        NativeServiceHealthProbe {
            endpoint_reachable: true,
            node_role_readable: true,
        }
    }

    #[test]
    fn requires_health_before_session_handoff() {
        let mut supervisor = NativeServiceSupervisor::new(2);
        supervisor.start();

        assert_eq!(
            supervisor.record_session_handoff(true),
            Err(NativeServiceSupervisorError::EndpointNotHealthy)
        );
        assert_eq!(supervisor.record_health_probe(service_probe()), Ok(()));
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::EndpointHealthy
        );
        assert_eq!(
            supervisor.record_session_handoff(false),
            Err(NativeServiceSupervisorError::SessionNotBound)
        );
        assert_eq!(supervisor.record_session_handoff(true), Ok(()));
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::SessionHandoffReady
        );
    }

    #[test]
    fn classifies_retryable_failures_until_budget_is_exhausted() {
        let mut supervisor = NativeServiceSupervisor::new(1);
        supervisor.start();

        let first = supervisor.record_failure(NativeServiceFailureKind::BindFailed, "port_busy");
        assert!(first.retryable);
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::Restarting
        );

        let second = supervisor.record_failure(NativeServiceFailureKind::HealthProbeFailed, "dead");
        assert!(!second.retryable);
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::Offline
        );
    }

    #[test]
    fn keeps_session_handoff_failure_fatal() {
        let mut supervisor = NativeServiceSupervisor::new(3);
        supervisor.start();

        let offline = supervisor.record_failure(
            NativeServiceFailureKind::SessionHandoffFailed,
            "session_missing",
        );

        assert!(!offline.retryable);
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::Offline
        );
    }

    #[test]
    fn records_external_offline_state_without_losing_reason() {
        let mut supervisor = NativeServiceSupervisor::new(2);
        supervisor.start();
        supervisor
            .record_health_probe(service_probe())
            .expect("probe");
        supervisor.record_session_handoff(true).expect("session");

        supervisor.record_service_offline(NativeServiceOffline {
            reason: "service_dead".to_string(),
            retryable: true,
        });

        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.state, NativeServiceSupervisorState::Restarting);
        assert_eq!(snapshot.restart_attempt, 1);
        assert_eq!(
            snapshot.offline,
            Some(NativeServiceOffline {
                reason: "service_dead".to_string(),
                retryable: true,
            })
        );
    }

    #[test]
    fn external_offline_retryability_respects_restart_budget() {
        let mut supervisor = NativeServiceSupervisor::new(1);

        let first = supervisor.record_service_offline(NativeServiceOffline {
            reason: "probe_failed".to_string(),
            retryable: true,
        });
        let second = supervisor.record_service_offline(NativeServiceOffline {
            reason: "still_dead".to_string(),
            retryable: true,
        });

        assert!(first.retryable);
        assert!(!second.retryable);
        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.state, NativeServiceSupervisorState::Offline);
        assert_eq!(snapshot.restart_attempt, 1);
        assert_eq!(
            snapshot.offline,
            Some(NativeServiceOffline {
                reason: "still_dead".to_string(),
                retryable: false,
            })
        );
    }
}
