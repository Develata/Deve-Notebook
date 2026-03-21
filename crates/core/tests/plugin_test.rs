#[cfg(test)]
mod tests {
    use deve_core::plugin::loader::PluginLoader;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    #[cfg(unix)]
    use tempfile::tempdir;

    #[test]
    fn test_rhai_http_plugin() {
        let plugin_dir = PathBuf::from("tests/plugins");
        // Ensure manifest exists
        if !plugin_dir.join("manifest.json").exists() {
            // Skip test if environment not set up (e.g. CI without network)
            return;
        }

        let loader = PluginLoader::new(plugin_dir.clone());
        // Load specific plugin
        let _runtime = loader
            .load_plugin(&plugin_dir)
            .expect("Failed to load plugin");

        // Execute run_test
        // Note: The script executes immediately on load.
        // To test specific function, we need to call it.
        // But main.rhai above calls run_test() at top level.

        // Let's call a specific function if we change the script structure
        // Or check the return value of the script execution?
        // Rhai run_ast returns the result of the last statement.
    }

    #[cfg(unix)]
    #[test]
    fn plugin_loader_fails_closed_on_unstatable_plugin_root() {
        let dir = tempdir().expect("tempdir");
        let blocked = dir.path().join("blocked");
        let plugin_root = blocked.join("plugins");
        std::fs::create_dir_all(&plugin_root).expect("mkdir");
        let original = std::fs::metadata(&blocked).expect("metadata").permissions();
        let mut perms = original.clone();
        perms.set_mode(0o000);
        std::fs::set_permissions(&blocked, perms).expect("chmod 000");

        let loader = PluginLoader::new(plugin_root);
        let err = match loader.load_all() {
            Ok(_) => panic!("unstatable plugin root must fail closed"),
            Err(err) => err,
        };

        std::fs::set_permissions(&blocked, original).expect("restore perms");
        assert!(
            err.to_string().contains("Failed to stat plugin directory")
                || err.to_string().contains("Permission denied")
        );
    }
}
