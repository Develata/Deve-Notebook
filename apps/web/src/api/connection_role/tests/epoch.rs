//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!

use super::super::*;
use leptos::prelude::{GetUntracked, Set, signal};

#[test]
fn stale_node_role_probe_results_do_not_mutate_current_connection() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let lifecycle = ConnectionLifecycle::new();
    let (node_role, set_node_role) = signal("main".to_string());
    let (source_control_git_bridge, set_source_control_git_bridge) = signal("mirror".to_string());
    let (host_file_copy_absolute_path, set_host_file_copy_absolute_path) = signal(true);
    let (host_file_reveal_in_system_explorer, set_host_file_reveal_in_system_explorer) =
        signal(true);
    let (probe_failed, set_probe_failed) = signal(false);
    let (connection_epoch, set_connection_epoch) = signal(2u64);

    assert!(!apply_node_role_probe_failure(
        &lifecycle,
        set_node_role,
        set_source_control_git_bridge,
        set_host_file_copy_absolute_path,
        set_host_file_reveal_in_system_explorer,
        set_probe_failed,
        connection_epoch,
        1,
    ));
    assert_eq!(node_role.get_untracked(), "main");
    assert_eq!(source_control_git_bridge.get_untracked(), "mirror");
    assert!(host_file_copy_absolute_path.get_untracked());
    assert!(host_file_reveal_in_system_explorer.get_untracked());
    assert!(!probe_failed.get_untracked());

    assert!(apply_node_role_probe_failure(
        &lifecycle,
        set_node_role,
        set_source_control_git_bridge,
        set_host_file_copy_absolute_path,
        set_host_file_reveal_in_system_explorer,
        set_probe_failed,
        connection_epoch,
        2,
    ));
    assert_eq!(node_role.get_untracked(), "");
    assert_eq!(source_control_git_bridge.get_untracked(), "unknown");
    assert!(!host_file_copy_absolute_path.get_untracked());
    assert!(!host_file_reveal_in_system_explorer.get_untracked());
    assert!(probe_failed.get_untracked());

    set_connection_epoch.set(3);
    assert!(!apply_node_role_probe_success(
        &lifecycle,
        set_node_role,
        set_source_control_git_bridge,
        set_host_file_copy_absolute_path,
        set_host_file_reveal_in_system_explorer,
        set_probe_failed,
        connection_epoch,
        2,
        NodeRoleProbeResult {
            summary: "proxy".to_string(),
            source_control_git_bridge: "off".to_string(),
            host_file_copy_absolute_path: true,
            host_file_reveal_in_system_explorer: true,
        },
    ));
    assert_eq!(node_role.get_untracked(), "");
    assert_eq!(source_control_git_bridge.get_untracked(), "unknown");
    assert!(!host_file_copy_absolute_path.get_untracked());
    assert!(!host_file_reveal_in_system_explorer.get_untracked());
    assert!(probe_failed.get_untracked());

    assert!(apply_node_role_probe_success(
        &lifecycle,
        set_node_role,
        set_source_control_git_bridge,
        set_host_file_copy_absolute_path,
        set_host_file_reveal_in_system_explorer,
        set_probe_failed,
        connection_epoch,
        3,
        NodeRoleProbeResult {
            summary: "main".to_string(),
            source_control_git_bridge: "off".to_string(),
            host_file_copy_absolute_path: true,
            host_file_reveal_in_system_explorer: false,
        },
    ));
    assert_eq!(node_role.get_untracked(), "main");
    assert_eq!(source_control_git_bridge.get_untracked(), "off");
    assert!(host_file_copy_absolute_path.get_untracked());
    assert!(!host_file_reveal_in_system_explorer.get_untracked());
    assert!(!probe_failed.get_untracked());
}
