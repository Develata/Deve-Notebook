//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 08_auth#unauthorized-disconnected-ui
//!

use super::*;
use crate::api::write_gate::WriterReadyResetSignals;
use leptos::prelude::GetUntracked;
use std::collections::VecDeque;

#[test]
fn non_connected_status_resets_node_role_runtime_summary() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (status, set_status) = signal(ConnectionStatus::Connected);
    let (msg_seq, set_msg_seq) = signal(0u64);
    let (_msg_queue, set_msg_queue) =
        signal(VecDeque::<(u64, u64, deve_core::protocol::ServerMessage)>::new());
    let (connection_epoch, set_connection_epoch) = signal(1u64);
    let (_endpoint, set_endpoint) = signal("ws://127.0.0.1:3001/ws".to_string());
    let (node_role, set_node_role) = signal("main".to_string());
    let (source_control_authority, set_source_control_authority) =
        signal("stale-authority".to_string());
    let (host_file_copy_absolute_path, set_host_file_copy_absolute_path) = signal(true);
    let (host_file_reveal_in_system_explorer, set_host_file_reveal_in_system_explorer) =
        signal(true);
    let (node_role_probe_failed, set_node_role_probe_failed) = signal(true);
    let (writer_ready_repo_id, set_writer_ready_repo_id) = signal(Some("repo-a".to_string()));
    let (writer_ready_scope_nonce, set_writer_ready_scope_nonce) = signal(Some(7u64));
    let (writer_client_id, set_writer_client_id) = signal(Some(9u64));

    let signals = ConnectionManagerSignals {
        lifecycle: ConnectionLifecycle::new(),
        set_status,
        set_msg_seq,
        set_msg_queue,
        current_connection_epoch: connection_epoch,
        set_connection_epoch,
        set_endpoint,
        set_node_role,
        set_source_control_authority,
        set_host_file_copy_absolute_path,
        set_host_file_reveal_in_system_explorer,
        set_node_role_probe_failed,
        writer_ready_reset: WriterReadyResetSignals::new(
            set_writer_ready_repo_id,
            set_writer_ready_scope_nonce,
            set_writer_client_id,
        ),
    };

    assert!(try_set_connection_status(
        &signals,
        ConnectionStatus::Disconnected
    ));

    assert_eq!(status.get_untracked(), ConnectionStatus::Disconnected);
    assert_eq!(node_role.get_untracked(), "");
    assert_eq!(source_control_authority.get_untracked(), "unknown");
    assert!(!host_file_copy_absolute_path.get_untracked());
    assert!(!host_file_reveal_in_system_explorer.get_untracked());
    assert!(!node_role_probe_failed.get_untracked());
    assert!(writer_ready_repo_id.get_untracked().is_none());
    assert!(writer_ready_scope_nonce.get_untracked().is_none());
    assert!(writer_client_id.get_untracked().is_none());
    assert_eq!(msg_seq.get_untracked(), 0);
}
