#[cfg(test)]
mod tests {
    use deve_core::plugin::loader::PluginLoader;
    use std::path::PathBuf;

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
        let mut runtime = loader
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
}
