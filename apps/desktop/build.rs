fn main() {
    #[cfg(feature = "native-packaging")]
    tauri_build::try_build(
        tauri_build::Attributes::new().plugin(
            "deve-native-backend-commands",
            tauri_build::InlinedPlugin::new()
                .commands(&[
                    "native_backend_get_config",
                    "native_backend_validate_remote",
                    "native_backend_save_remote",
                ])
                .default_permission(tauri_build::DefaultPermissionRule::AllowAllCommands),
        ),
    )
    .expect("failed to build Desktop Tauri application");
}
