//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!

use super::super::*;
use leptos::prelude::{GetUntracked, ReadSignal, Set, WriteSignal, signal};

#[derive(Clone, Copy)]
struct ProbeSetters {
    set_node_role: WriteSignal<String>,
    set_source_control_authority: WriteSignal<String>,
    set_host_file_copy_absolute_path: WriteSignal<bool>,
    set_host_file_reveal_in_system_explorer: WriteSignal<bool>,
    set_watcher_health: WriteSignal<WatcherHealthSnapshot>,
    set_node_role_probe_failed: WriteSignal<bool>,
}

fn probe_context(
    setters: ProbeSetters,
    current_connection_epoch: ReadSignal<u64>,
    probe_connection_epoch: u64,
) -> NodeRoleProbeContext {
    NodeRoleProbeContext {
        set_node_role: setters.set_node_role,
        set_source_control_authority: setters.set_source_control_authority,
        set_host_file_copy_absolute_path: setters.set_host_file_copy_absolute_path,
        set_host_file_reveal_in_system_explorer: setters.set_host_file_reveal_in_system_explorer,
        set_watcher_health: setters.set_watcher_health,
        set_node_role_probe_failed: setters.set_node_role_probe_failed,
        current_connection_epoch,
        probe_connection_epoch,
    }
}

#[test]
fn stale_node_role_probe_results_do_not_mutate_current_connection() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let lifecycle = ConnectionLifecycle::new();
    let (node_role, set_node_role) = signal("main".to_string());
    let (source_control_authority, set_source_control_authority) = signal("ngit".to_string());
    let (host_file_copy_absolute_path, set_host_file_copy_absolute_path) = signal(true);
    let (host_file_reveal_in_system_explorer, set_host_file_reveal_in_system_explorer) =
        signal(true);
    let (probe_failed, set_probe_failed) = signal(false);
    let (watcher_health, set_watcher_health) = signal(WatcherHealthSnapshot {
        status: WatcherHealthStatus::Healthy,
        expected: 1,
        running: 1,
        unavailable: 0,
    });
    let (connection_epoch, set_connection_epoch) = signal(2u64);
    let setters = ProbeSetters {
        set_node_role,
        set_source_control_authority,
        set_host_file_copy_absolute_path,
        set_host_file_reveal_in_system_explorer,
        set_watcher_health,
        set_node_role_probe_failed: set_probe_failed,
    };

    assert!(!apply_node_role_probe_failure(
        &lifecycle,
        probe_context(setters, connection_epoch, 1),
    ));
    assert_eq!(node_role.get_untracked(), "main");
    assert_eq!(source_control_authority.get_untracked(), "ngit");
    assert!(host_file_copy_absolute_path.get_untracked());
    assert!(host_file_reveal_in_system_explorer.get_untracked());
    assert!(!probe_failed.get_untracked());
    assert_eq!(
        watcher_health.get_untracked().status,
        WatcherHealthStatus::Healthy
    );

    assert!(apply_node_role_probe_failure(
        &lifecycle,
        probe_context(setters, connection_epoch, 2),
    ));
    assert_eq!(node_role.get_untracked(), "");
    assert_eq!(source_control_authority.get_untracked(), "unknown");
    assert!(!host_file_copy_absolute_path.get_untracked());
    assert!(!host_file_reveal_in_system_explorer.get_untracked());
    assert!(probe_failed.get_untracked());
    assert_eq!(
        watcher_health.get_untracked(),
        WatcherHealthSnapshot::default()
    );

    set_connection_epoch.set(3);
    assert!(!apply_node_role_probe_success(
        &lifecycle,
        probe_context(setters, connection_epoch, 2),
        NodeRoleProbeResult {
            summary: "proxy".to_string(),
            source_control_authority: "ngit".to_string(),
            host_file_copy_absolute_path: true,
            host_file_reveal_in_system_explorer: true,
            watcher_health: WatcherHealthSnapshot {
                status: WatcherHealthStatus::Degraded,
                expected: 2,
                running: 1,
                unavailable: 1,
            },
        },
    ));
    assert_eq!(node_role.get_untracked(), "");
    assert_eq!(source_control_authority.get_untracked(), "unknown");
    assert!(!host_file_copy_absolute_path.get_untracked());
    assert!(!host_file_reveal_in_system_explorer.get_untracked());
    assert!(probe_failed.get_untracked());

    assert!(apply_node_role_probe_success(
        &lifecycle,
        probe_context(setters, connection_epoch, 3),
        NodeRoleProbeResult {
            summary: "main".to_string(),
            source_control_authority: "ngit".to_string(),
            host_file_copy_absolute_path: true,
            host_file_reveal_in_system_explorer: false,
            watcher_health: WatcherHealthSnapshot {
                status: WatcherHealthStatus::Healthy,
                expected: 1,
                running: 1,
                unavailable: 0,
            },
        },
    ));
    assert_eq!(node_role.get_untracked(), "main");
    assert_eq!(source_control_authority.get_untracked(), "ngit");
    assert!(host_file_copy_absolute_path.get_untracked());
    assert!(!host_file_reveal_in_system_explorer.get_untracked());
    assert!(!probe_failed.get_untracked());
    assert_eq!(
        watcher_health.get_untracked().status,
        WatcherHealthStatus::Healthy
    );
}
