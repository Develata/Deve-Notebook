//! plan_ref:
//!   - 17_tech_stack#native-packaging-dependency-gate
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!
fn main() {
    #[cfg(feature = "native-packaging")]
    tauri_build::try_build(
        tauri_build::Attributes::new().plugin(
            "deve-native-backend-commands",
            tauri_build::InlinedPlugin::new()
                .commands(&[
                    "native_backend_get_config",
                    "native_backend_get_service_state",
                    "native_backend_get_recovery_state",
                    "native_backend_webview_session_bridge_ready",
                    "native_backend_prepare_webview_session",
                    "native_backend_debug_stop_transport",
                    "native_backend_debug_request_exit",
                    "native_backend_validate_remote",
                    "native_backend_save_remote",
                ])
                .default_permission(tauri_build::DefaultPermissionRule::AllowAllCommands),
        ),
    )
    .expect("failed to build Mobile Tauri application");
}
