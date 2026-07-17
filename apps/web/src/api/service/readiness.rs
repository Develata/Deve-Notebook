//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 08_auth#unauthorized-handling
//!   - 08_auth#unauthorized-disconnected-ui
//!   - 09_web_thin_client_ledger#write-readiness
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!

use deve_core::native_adapter::NativeRuntimeReadiness;
use leptos::prelude::*;

use super::{ConnectionStatus, WorkspaceIngestionBlocker, WsService};
use crate::api::WatcherHealthSnapshot;
use crate::api::write_gate::WriterReadyResetSignals;
use crate::api::writer_id::derive_writer_client_id;

#[derive(Clone, Debug)]
pub(super) struct NativeRuntimeConnectionState {
    pub status: ConnectionStatus,
    pub node_role: String,
    pub node_role_probe_failed: bool,
    pub ready_repo_id: Option<String>,
    pub ready_scope_nonce: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NativeRuntimeReadinessTarget<'a> {
    pub repo_id: Option<&'a str>,
    pub scope_nonce: Option<u64>,
    pub handshake_ready: bool,
}

pub(crate) fn is_current_connection_message(message_epoch: u64, current_epoch: u64) -> bool {
    message_epoch == current_epoch
}

pub(super) fn writer_ready_matches(
    ready_repo_id: Option<&str>,
    ready_scope_nonce: Option<u64>,
    repo_id: Option<&str>,
    scope_nonce: Option<u64>,
) -> bool {
    match (ready_repo_id, ready_scope_nonce, repo_id, scope_nonce) {
        (Some(ready_repo_id), Some(ready_scope_nonce), Some(repo_id), Some(scope_nonce)) => {
            ready_repo_id == repo_id && ready_scope_nonce == scope_nonce
        }
        _ => false,
    }
}

fn scope_nonce_current_matches(
    ready_repo_id: Option<&str>,
    ready_scope_nonce: Option<u64>,
    repo_id: Option<&str>,
    scope_nonce: Option<u64>,
) -> bool {
    writer_ready_matches(ready_repo_id, ready_scope_nonce, repo_id, scope_nonce)
}

pub(super) fn native_runtime_readiness_from_parts(
    state: NativeRuntimeConnectionState,
    target: NativeRuntimeReadinessTarget<'_>,
) -> NativeRuntimeReadiness {
    NativeRuntimeReadiness {
        endpoint_reachable: state.status == ConnectionStatus::Connected,
        auth_status_valid: !matches!(
            state.status,
            ConnectionStatus::Unauthorized
                | ConnectionStatus::NativeBootstrapInvalid
                | ConnectionStatus::NativeSessionPending
        ),
        node_role_readable: !state.node_role_probe_failed && !state.node_role.trim().is_empty(),
        repo_handshake_complete: target.handshake_ready,
        writer_ready: writer_ready_matches(
            state.ready_repo_id.as_deref(),
            state.ready_scope_nonce,
            target.repo_id,
            target.scope_nonce,
        ),
        scope_nonce_current: scope_nonce_current_matches(
            state.ready_repo_id.as_deref(),
            state.ready_scope_nonce,
            target.repo_id,
            target.scope_nonce,
        ),
    }
}

impl WsService {
    pub fn mark_unauthorized(&self) {
        self.clear_writer_ready();
        self.reset_node_role_state(false);
        self.set_status.set(ConnectionStatus::Unauthorized);
    }

    pub fn mark_writer_ready(&self, repo_id: impl Into<String>, scope_nonce: u64, peer_id: &str) {
        self.set_writer_ready_repo_id.set(Some(repo_id.into()));
        self.set_writer_ready_scope_nonce.set(Some(scope_nonce));
        self.set_writer_client_id.set(Some(derive_writer_client_id(
            peer_id,
            self.writer_session_nonce,
        )));
    }

    pub fn clear_writer_ready(&self) {
        let _ = self.writer_ready_reset_signals().clear();
    }

    pub(crate) fn begin_foreground_reprobe(&self) {
        self.clear_writer_ready();
        self.reset_node_role_state(true);
        self.set_status.set(ConnectionStatus::NativeReprobeRequired);
    }

    pub(crate) fn complete_foreground_node_role_reprobe(
        &self,
        summary: impl Into<String>,
        source_control_authority: impl Into<String>,
        host_file_copy_absolute_path: bool,
        host_file_reveal_in_system_explorer: bool,
        watcher_health: WatcherHealthSnapshot,
    ) {
        self.set_node_role.set(summary.into());
        self.set_source_control_authority
            .set(source_control_authority.into());
        self.set_host_file_copy_absolute_path
            .set(host_file_copy_absolute_path);
        self.set_host_file_reveal_in_system_explorer
            .set(host_file_reveal_in_system_explorer);
        self.set_watcher_health.set(watcher_health);
        self.set_node_role_probe_failed.set(false);
        self.set_status.set(ConnectionStatus::Connected);
    }

    pub(crate) fn fail_foreground_node_role_reprobe(&self) {
        self.reset_node_role_state(true);
    }

    pub(crate) fn mark_native_service_offline(&self) {
        self.clear_writer_ready();
        self.reset_node_role_state(true);
        self.set_status.set(ConnectionStatus::NativeServiceOffline);
    }

    fn reset_node_role_state(&self, probe_failed: bool) {
        self.set_node_role.set(String::new());
        self.set_source_control_authority.set("unknown".to_string());
        self.set_host_file_copy_absolute_path.set(false);
        self.set_host_file_reveal_in_system_explorer.set(false);
        self.set_watcher_health
            .set(WatcherHealthSnapshot::default());
        self.set_node_role_probe_failed.set(probe_failed);
    }

    pub fn writer_ready_for(&self, repo_id: Option<&str>, scope_nonce: Option<u64>) -> bool {
        let ready_repo_id = self.writer_ready_repo_id.get_untracked();
        writer_ready_matches(
            ready_repo_id.as_deref(),
            self.writer_ready_scope_nonce.get_untracked(),
            repo_id,
            scope_nonce,
        )
    }

    pub(crate) fn mark_workspace_ingestion_unavailable(
        &self,
        repo_id: impl Into<String>,
        scope_nonce: u64,
    ) {
        self.set_workspace_ingestion_blocker
            .set(Some(WorkspaceIngestionBlocker {
                connection_epoch: self.connection_epoch.get_untracked(),
                repo_id: repo_id.into(),
                scope_nonce,
            }));
    }

    pub(crate) fn workspace_ingestion_blocked_for(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
    ) -> bool {
        workspace_ingestion_blocker_matches(
            self.workspace_ingestion_blocker.get(),
            self.connection_epoch.get(),
            repo_id,
            scope_nonce,
        )
    }

    pub(crate) fn workspace_ingestion_blocked_for_untracked(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
    ) -> bool {
        workspace_ingestion_blocker_matches(
            self.workspace_ingestion_blocker.get_untracked(),
            self.connection_epoch.get_untracked(),
            repo_id,
            scope_nonce,
        )
    }

    pub fn native_runtime_readiness_for(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
        handshake_ready: bool,
    ) -> NativeRuntimeReadiness {
        native_runtime_readiness_from_parts(
            NativeRuntimeConnectionState {
                status: self.status.get(),
                node_role: self.node_role.get(),
                node_role_probe_failed: self.node_role_probe_failed.get(),
                ready_repo_id: self.writer_ready_repo_id.get(),
                ready_scope_nonce: self.writer_ready_scope_nonce.get(),
            },
            NativeRuntimeReadinessTarget {
                repo_id,
                scope_nonce,
                handshake_ready,
            },
        )
    }

    pub fn native_runtime_readiness_for_untracked(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
        handshake_ready: bool,
    ) -> NativeRuntimeReadiness {
        native_runtime_readiness_from_parts(
            NativeRuntimeConnectionState {
                status: self.status.get_untracked(),
                node_role: self.node_role.get_untracked(),
                node_role_probe_failed: self.node_role_probe_failed.get_untracked(),
                ready_repo_id: self.writer_ready_repo_id.get_untracked(),
                ready_scope_nonce: self.writer_ready_scope_nonce.get_untracked(),
            },
            NativeRuntimeReadinessTarget {
                repo_id,
                scope_nonce,
                handshake_ready,
            },
        )
    }

    pub fn writer_client_id_for(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
    ) -> Option<u64> {
        match (self.writer_client_id.get_untracked(), repo_id, scope_nonce) {
            (Some(client_id), Some(repo_id), Some(scope_nonce))
                if self.writer_ready_for(Some(repo_id), Some(scope_nonce)) =>
            {
                Some(client_id)
            }
            _ => None,
        }
    }

    pub(super) fn writer_ready_reset_signals(&self) -> WriterReadyResetSignals {
        WriterReadyResetSignals::new(
            self.set_writer_ready_repo_id,
            self.set_writer_ready_scope_nonce,
            self.set_writer_client_id,
        )
    }
}

fn workspace_ingestion_blocker_matches(
    blocker: Option<WorkspaceIngestionBlocker>,
    connection_epoch: u64,
    repo_id: Option<&str>,
    scope_nonce: Option<u64>,
) -> bool {
    matches!(
        (blocker, repo_id, scope_nonce),
        (Some(blocker), Some(repo_id), Some(scope_nonce))
            if blocker.connection_epoch == connection_epoch
                && blocker.repo_id == repo_id
                && blocker.scope_nonce == scope_nonce
    )
}

#[cfg(test)]
mod workspace_ingestion_tests {
    use super::*;

    #[test]
    fn workspace_ingestion_blocker_requires_exact_epoch_repo_and_scope() {
        let blocker = WorkspaceIngestionBlocker {
            connection_epoch: 3,
            repo_id: "repo-a".into(),
            scope_nonce: 7,
        };

        assert!(workspace_ingestion_blocker_matches(
            Some(blocker.clone()),
            3,
            Some("repo-a"),
            Some(7)
        ));
        assert!(!workspace_ingestion_blocker_matches(
            Some(blocker.clone()),
            4,
            Some("repo-a"),
            Some(7)
        ));
        assert!(!workspace_ingestion_blocker_matches(
            Some(blocker.clone()),
            3,
            Some("repo-b"),
            Some(7)
        ));
        assert!(!workspace_ingestion_blocker_matches(
            Some(blocker),
            3,
            Some("repo-a"),
            Some(8)
        ));
    }
}
