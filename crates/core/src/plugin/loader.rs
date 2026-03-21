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
use crate::plugin::manifest::PluginManifest;
#[cfg(not(target_arch = "wasm32"))]
use crate::plugin::runtime::{PluginRuntime, RhaiRuntime};
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Context, Result};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

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

        if !self
            .plugin_dir
            .try_exists()
            .with_context(|| format!("Failed to stat plugin directory: {:?}", self.plugin_dir))?
        {
            return Ok(plugins);
        }

        for entry in fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type()?.is_dir() {
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

    pub fn load_plugin(&self, path: &Path) -> Result<Box<dyn PluginRuntime>> {
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

        // 3. Initialize Runtime (传递插件目录路径以支持模块解析)
        let mut runtime = RhaiRuntime::new(manifest.clone(), path.to_path_buf());
        runtime.load(manifest, &script_content)?;

        Ok(Box::new(runtime))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
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

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), unix))]
    fn load_all_fails_closed_when_plugin_dir_is_unstatable() {
        let dir = tempdir().expect("tempdir");
        let original = fs::metadata(dir.path()).expect("metadata").permissions();
        let mut blocked = original.clone();
        blocked.set_mode(0o000);
        fs::set_permissions(dir.path(), blocked).expect("chmod 000");

        let loader = PluginLoader::new(dir.path().to_path_buf());
        let err = match loader.load_all() {
            Ok(_) => panic!("unstatable plugin dir must fail closed"),
            Err(err) => err,
        };

        fs::set_permissions(dir.path(), original).expect("restore perms");
        assert!(
            err.to_string().contains("Failed to stat plugin directory")
                || err.to_string().contains("Permission denied")
        );
    }
}
