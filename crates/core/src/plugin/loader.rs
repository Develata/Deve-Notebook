// crates\core\src\plugin
//! # Plugin Loader (插件加载器)
//!
//! **架构作用**:
//! 负责从磁盘目录扫描、读取并初始化插件。
//!
//! **核心功能清单**:
//! - `PluginLoader`: 管理插件加载流程。
//! - `scan_plugins`: 遍历指定目录，寻找 `manifest.json`。
//! - `load_plugin`: 读取 Manifest 与 Entry Script，创建 Runtime 实例。
//!
//! **类型**: Core MUST (核心必选)

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Result, Context};
#[cfg(not(target_arch = "wasm32"))]
use crate::plugin::manifest::PluginManifest;
#[cfg(not(target_arch = "wasm32"))]
use crate::plugin::runtime::{PluginRuntime, RhaiRuntime};

#[cfg(not(target_arch = "wasm32"))]
pub struct PluginLoader {
    plugin_dir: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl PluginLoader {
    pub fn new(plugin_dir: PathBuf) -> Self {
        Self { plugin_dir }
    }

    /// Scan and load all plugins in the plugin directory.
    pub fn load_all(&self) -> Result<Vec<Box<dyn PluginRuntime>>> {
        let mut plugins = Vec::new();

        if !self.plugin_dir.exists() {
             return Ok(plugins);
        }

        for entry in fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                match self.load_plugin(&path) {
                    Ok(runtime) => {
                        println!("Loaded plugin: {}", runtime.manifest().name);
                        plugins.push(runtime);
                    }
                    Err(e) => {
                        eprintln!("Failed to load plugin at {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(plugins)
    }

    fn load_plugin(&self, path: &Path) -> Result<Box<dyn PluginRuntime>> {
        // 1. Read manifest.json
        let manifest_path = path.join("manifest.json");
        let manifest_content = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Missing manifest.json in {:?}", path))?;
        
        let manifest: PluginManifest = serde_json::from_str(&manifest_content)
            .with_context(|| "Failed to parse manifest.json")?;

        // 2. Read entry script
        let entry_path = path.join(&manifest.entry);
        let script_content = fs::read_to_string(&entry_path)
            .with_context(|| format!("Missing entry script '{}' in {:?}", manifest.entry, path))?;

        // 3. Initialize Runtime
        let mut runtime = RhaiRuntime::new(manifest.clone());
        runtime.load(manifest, &script_content)?;

        Ok(Box::new(runtime))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_plugin_loader() {
        // 1. Setup temp plugin directory
        let dir = tempdir().unwrap();
        let plugin_dir = dir.path().join("my-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        // 2. Create manifest.json
        let manifest_content = r#"{
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "entry": "index.rhai",
            "capabilities": {
                "allow_env": ["USER"]
            }
        }"#;
        fs::write(plugin_dir.join("manifest.json"), manifest_content).unwrap();

        // 3. Create entry script
        let script_content = r#"
            fn hello() {
                return "world";
            }
        "#;
        fs::write(plugin_dir.join("index.rhai"), script_content).unwrap();

        // 4. Load
        let loader = PluginLoader::new(dir.path().to_path_buf());
        let plugins = loader.load_all().expect("Failed to load plugins");

        assert_eq!(plugins.len(), 1);
        let plugin = &plugins[0];
        assert_eq!(plugin.manifest().id, "test-plugin");
        
        let res = plugin.call("hello", vec![]).expect("Failed to call");
        assert_eq!(res.clone().into_string().unwrap(), "world");
    }
}
