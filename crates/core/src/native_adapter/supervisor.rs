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

#[cfg(test)]
mod tests {
    use super::*;

    fn service_probe() -> NativeServiceHealthProbe {
        NativeServiceHealthProbe {
            endpoint_reachable: true,
            node_role_readable: true,
        }
    }

    fn service_endpoint() -> super::super::NativeEndpointReady {
        super::super::NativeEndpointReady {
            http_base: "http://127.0.0.1:3001".to_string(),
            ws_base: "ws://127.0.0.1:3001".to_string(),
            node_role: "native-main".to_string(),
            session_bound: false,
        }
    }

    fn ready_process_snapshot() -> NativeProcessAdapterSnapshot {
        let mut process = super::super::NativeProcessAdapter::default();
        process
            .bind_existing_endpoint(service_endpoint())
            .expect("endpoint");
        process.bind_session(true).expect("session")
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
        let retryable_failure =
            supervisor.record_failure(NativeServiceFailureKind::ProcessExited, "process_exited");
        assert!(!retryable_failure.retryable);
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::Offline
        );
        assert_eq!(
            supervisor.snapshot().offline,
            Some(NativeServiceOffline {
                reason: "session_missing".to_string(),
                retryable: false,
            })
        );
    }

    #[test]
    fn records_external_offline_state_without_losing_reason() {
        let mut supervisor = NativeServiceSupervisor::new(2);
        supervisor.start();
        assert_eq!(
            supervisor.record_process_snapshot(&ready_process_snapshot()),
            NativeServiceSupervisorObservation::SessionHandoffReady
        );

        supervisor.record_service_offline(NativeServiceOffline {
            reason: "service_dead".to_string(),
            retryable: true,
        });

        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.state, NativeServiceSupervisorState::Restarting);
        assert_eq!(snapshot.restart_attempt, 0);
        assert_eq!(
            snapshot.offline,
            Some(NativeServiceOffline {
                reason: "service_dead".to_string(),
                retryable: true,
            })
        );
    }

    #[test]
    fn process_snapshot_drives_health_and_session_handoff() {
        let mut supervisor = NativeServiceSupervisor::new(2);
        supervisor.start();
        let mut process = super::super::NativeProcessAdapter::default();
        let endpoint = super::super::NativeEndpointReady {
            http_base: "http://127.0.0.1:3001".to_string(),
            ws_base: "ws://127.0.0.1:3001".to_string(),
            node_role: "native-main".to_string(),
            session_bound: false,
        };

        let endpoint_snapshot = process.bind_existing_endpoint(endpoint).expect("endpoint");
        assert_eq!(
            supervisor.record_process_snapshot(&endpoint_snapshot),
            NativeServiceSupervisorObservation::EndpointHealthy
        );
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::EndpointHealthy
        );

        let session_snapshot = process.bind_session(true).expect("session");
        assert_eq!(
            supervisor.record_process_snapshot(&session_snapshot),
            NativeServiceSupervisorObservation::SessionHandoffReady
        );
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::SessionHandoffReady
        );
    }

    #[test]
    fn process_probe_timeout_snapshot_consumes_retry_budget() {
        let mut supervisor = NativeServiceSupervisor::new(1);
        supervisor.start();
        let mut process = super::super::NativeProcessAdapter::default();
        process
            .bind_existing_endpoint(super::super::NativeEndpointReady {
                http_base: "http://127.0.0.1:3001".to_string(),
                ws_base: "ws://127.0.0.1:3001".to_string(),
                node_role: "native-main".to_string(),
                session_bound: false,
            })
            .expect("endpoint");

        let first_timeout = process.record_probe_timeout();
        assert_eq!(
            supervisor.record_process_snapshot(&first_timeout),
            NativeServiceSupervisorObservation::Offline(NativeServiceOffline {
                reason: "probe_failed".to_string(),
                retryable: true,
            })
        );
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::Restarting
        );
        assert_eq!(supervisor.snapshot().restart_attempt, 1);

        let second_timeout = process.record_probe_timeout();
        assert_eq!(
            supervisor.record_process_snapshot(&second_timeout),
            NativeServiceSupervisorObservation::Offline(NativeServiceOffline {
                reason: "probe_failed".to_string(),
                retryable: false,
            })
        );
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::Offline
        );
    }

    #[test]
    fn process_shutdown_snapshot_enters_restart_path() {
        let mut supervisor = NativeServiceSupervisor::new(2);
        supervisor.start();
        let mut process = super::super::NativeProcessAdapter::default();
        process
            .bind_existing_endpoint(super::super::NativeEndpointReady {
                http_base: "http://127.0.0.1:3001".to_string(),
                ws_base: "ws://127.0.0.1:3001".to_string(),
                node_role: "native-main".to_string(),
                session_bound: false,
            })
            .expect("endpoint");

        let stopped = process.record_process_stopped();

        assert_eq!(
            supervisor.record_process_snapshot(&stopped),
            NativeServiceSupervisorObservation::Offline(NativeServiceOffline {
                reason: "process_stopped".to_string(),
                retryable: true,
            })
        );
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::Restarting
        );
    }

    #[test]
    fn malformed_session_snapshot_is_fatal_offline() {
        let mut supervisor = NativeServiceSupervisor::new(2);
        supervisor.start();
        let snapshot = NativeProcessAdapterSnapshot {
            state: NativeProcessAdapterState::SessionHandoffReady,
            endpoint: Some(super::super::NativeEndpointReady {
                http_base: "http://127.0.0.1:3001".to_string(),
                ws_base: "ws://127.0.0.1:3001".to_string(),
                node_role: "native-main".to_string(),
                session_bound: false,
            }),
            health_probe: service_probe(),
            child_process_runtime_enabled: false,
            child_process_running: false,
            authority_writes_allowed: false,
        };

        assert_eq!(
            supervisor.record_process_snapshot(&snapshot),
            NativeServiceSupervisorObservation::Offline(NativeServiceOffline {
                reason: "session_not_bound".to_string(),
                retryable: false,
            })
        );
        assert_eq!(
            supervisor.snapshot().state,
            NativeServiceSupervisorState::Offline
        );
    }

    #[test]
    fn external_offline_observation_does_not_consume_restart_budget() {
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
        assert!(second.retryable);
        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.state, NativeServiceSupervisorState::Restarting);
        assert_eq!(snapshot.restart_attempt, 0);
        assert_eq!(
            snapshot.offline,
            Some(NativeServiceOffline {
                reason: "still_dead".to_string(),
                retryable: true,
            })
        );
    }

    #[test]
    fn external_offline_retryability_is_clamped_after_failure_budget_is_exhausted() {
        let mut supervisor = NativeServiceSupervisor::new(1);
        let failure = supervisor.record_failure(NativeServiceFailureKind::BindFailed, "port_busy");
        assert!(failure.retryable);

        let observed = supervisor.record_service_offline(NativeServiceOffline {
            reason: "still_dead".to_string(),
            retryable: true,
        });

        assert!(!observed.retryable);
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

    #[test]
    fn external_offline_observation_cannot_upgrade_terminal_offline() {
        let mut supervisor = NativeServiceSupervisor::new(3);
        let fatal = supervisor.record_failure(
            NativeServiceFailureKind::SessionHandoffFailed,
            "session_missing",
        );
        assert!(!fatal.retryable);

        let observed = supervisor.record_service_offline(NativeServiceOffline {
            reason: "service_dead".to_string(),
            retryable: true,
        });

        assert!(!observed.retryable);
        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.state, NativeServiceSupervisorState::Offline);
        assert_eq!(snapshot.restart_attempt, 0);
        assert_eq!(
            snapshot.offline,
            Some(NativeServiceOffline {
                reason: "session_missing".to_string(),
                retryable: false,
            })
        );
    }

    #[test]
    fn start_does_not_clear_terminal_offline() {
        let mut supervisor = NativeServiceSupervisor::new(3);
        let fatal = supervisor.record_failure(
            NativeServiceFailureKind::SessionHandoffFailed,
            "session_missing",
        );
        assert!(!fatal.retryable);

        supervisor.start();

        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.state, NativeServiceSupervisorState::Offline);
        assert_eq!(snapshot.restart_attempt, 0);
        assert_eq!(
            snapshot.offline,
            Some(NativeServiceOffline {
                reason: "session_missing".to_string(),
                retryable: false,
            })
        );
    }

    #[test]
    fn start_does_not_clear_budget_exhausted_offline() {
        let mut supervisor = NativeServiceSupervisor::new(1);
        let retryable = supervisor.record_failure(NativeServiceFailureKind::BindFailed, "first");
        assert!(retryable.retryable);
        let terminal = supervisor.record_failure(NativeServiceFailureKind::ProcessExited, "second");
        assert!(!terminal.retryable);

        supervisor.start();

        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.state, NativeServiceSupervisorState::Offline);
        assert_eq!(snapshot.restart_attempt, 1);
        assert_eq!(
            snapshot.offline,
            Some(NativeServiceOffline {
                reason: "second".to_string(),
                retryable: false,
            })
        );
    }

    #[test]
    fn start_clears_retryable_offline_without_resetting_budget() {
        let mut supervisor = NativeServiceSupervisor::new(3);
        let retryable =
            supervisor.record_failure(NativeServiceFailureKind::BindFailed, "port_busy");
        assert!(retryable.retryable);

        supervisor.start();

        let snapshot = supervisor.snapshot();
        assert_eq!(snapshot.state, NativeServiceSupervisorState::Starting);
        assert_eq!(snapshot.restart_attempt, 1);
        assert_eq!(snapshot.offline, None);
    }
}
