// crates\core\src\plugin
//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
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
use anyhow::{Context, Result, bail};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Component, Path, PathBuf};

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
        self.load_all_with_mode(false)
    }

    /// Scan and load all plugins in the plugin directory.
    ///
    /// Invariants:
    /// - `serve` 路径下，坏插件不能被静默忽略。
    /// - 任一插件加载失败时，调用方必须能显式得到错误。
    pub fn load_all_strict(&self) -> Result<Vec<Box<dyn PluginRuntime>>> {
        self.load_all_with_mode(true)
    }

    fn load_all_with_mode(&self, fail_closed: bool) -> Result<Vec<Box<dyn PluginRuntime>>> {
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
                match self
                    .load_plugin(&path)
                    .with_context(|| format!("Failed to load plugin at {:?}", path))
                {
                    Ok(runtime) => {
                        println!("Loaded plugin: {}", runtime.manifest().name);
                        plugins.push(runtime);
                    }
                    Err(e) => {
                        if fail_closed {
                            return Err(e);
                        }
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
        let entry_path = validate_plugin_entry(path, &manifest.entry)?;
        let script_content = fs::read_to_string(&entry_path)
            .with_context(|| format!("Missing entry script '{}' in {:?}", manifest.entry, path))?;

        // 3. Initialize Runtime (传递插件目录路径以支持模块解析)
        let mut runtime = RhaiRuntime::new(manifest.clone(), path.to_path_buf());
        runtime.load(manifest, &script_content)?;

        Ok(Box::new(runtime))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_plugin_entry(plugin_root: &Path, entry: &str) -> Result<PathBuf> {
    if entry.is_empty() {
        bail!("Invalid plugin entry: entry must not be empty");
    }
    if entry.contains('\\') {
        bail!(
            "Invalid plugin entry '{}': use forward-slash relative paths",
            entry
        );
    }

    let entry_path = Path::new(entry);
    if entry_path.is_absolute() {
        bail!(
            "Invalid plugin entry '{}': absolute paths are not allowed",
            entry
        );
    }

    let mut has_component = false;
    for component in entry_path.components() {
        has_component = true;
        if !matches!(component, Component::Normal(_)) {
            bail!(
                "Invalid plugin entry '{}': only normal relative path segments are allowed",
                entry
            );
        }
    }
    if !has_component {
        bail!("Invalid plugin entry: entry must name a Rhai script");
    }

    if entry_path.extension().and_then(|ext| ext.to_str()) != Some("rhai") {
        bail!(
            "Invalid plugin entry '{}': entry must be a .rhai script",
            entry
        );
    }

    let resolved = plugin_root.join(entry_path);
    if !resolved.starts_with(plugin_root) {
        bail!(
            "Invalid plugin entry '{}': entry must stay inside plugin directory",
            entry
        );
    }
    Ok(resolved)
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

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn load_all_strict_fails_closed_when_any_plugin_is_broken() {
        let dir = tempdir().expect("tempdir");

        let good = dir.path().join("good-plugin");
        fs::create_dir(&good).expect("mkdir good");
        fs::write(
            good.join("manifest.json"),
            r#"{
                "id": "good-plugin",
                "name": "Good Plugin",
                "version": "1.0.0",
                "entry": "index.rhai"
            }"#,
        )
        .expect("write good manifest");
        fs::write(good.join("index.rhai"), "fn hello() { \"ok\" }").expect("write good entry");

        let broken = dir.path().join("broken-plugin");
        fs::create_dir(&broken).expect("mkdir broken");
        fs::write(
            broken.join("manifest.json"),
            r#"{
                "id": "broken-plugin",
                "name": "Broken Plugin",
                "version": "1.0.0",
                "entry": "missing.rhai"
            }"#,
        )
        .expect("write broken manifest");

        let loader = PluginLoader::new(dir.path().to_path_buf());
        let err = match loader.load_all_strict() {
            Ok(_) => panic!("broken plugin must fail closed in strict mode"),
            Err(err) => err,
        };

        assert!(
            err.to_string().contains("Failed to load plugin at")
                || err.to_string().contains("Missing entry script")
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn load_plugin_rejects_entry_parent_traversal() {
        let dir = tempdir().expect("tempdir");
        let plugin_dir = dir.path().join("bad-plugin");
        fs::create_dir(&plugin_dir).expect("mkdir plugin");
        fs::write(
            plugin_dir.join("manifest.json"),
            r#"{
                "id": "bad-plugin",
                "name": "Bad Plugin",
                "version": "1.0.0",
                "entry": "../outside.rhai"
            }"#,
        )
        .expect("write manifest");
        fs::write(dir.path().join("outside.rhai"), "fn hello() { \"bad\" }")
            .expect("write outside");

        let loader = PluginLoader::new(dir.path().to_path_buf());
        let err = match loader.load_plugin(&plugin_dir) {
            Ok(_) => panic!("entry traversal must be rejected"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("Invalid plugin entry"));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn load_plugin_rejects_absolute_entry_path() {
        let dir = tempdir().expect("tempdir");
        let plugin_dir = dir.path().join("bad-plugin");
        fs::create_dir(&plugin_dir).expect("mkdir plugin");
        let outside = dir
            .path()
            .join("outside.rhai")
            .to_string_lossy()
            .replace('\\', "/");
        let manifest_content = serde_json::json!({
            "id": "bad-plugin",
            "name": "Bad Plugin",
            "version": "1.0.0",
            "entry": outside
        })
        .to_string();
        fs::write(plugin_dir.join("manifest.json"), manifest_content).expect("write manifest");

        let loader = PluginLoader::new(dir.path().to_path_buf());
        let err = match loader.load_plugin(&plugin_dir) {
            Ok(_) => panic!("absolute entry path must be rejected"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("absolute paths are not allowed"));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn load_plugin_rejects_backslash_entry_path() {
        let dir = tempdir().expect("tempdir");
        let plugin_dir = dir.path().join("bad-plugin");
        fs::create_dir(&plugin_dir).expect("mkdir plugin");
        fs::write(
            plugin_dir.join("manifest.json"),
            r#"{
                "id": "bad-plugin",
                "name": "Bad Plugin",
                "version": "1.0.0",
                "entry": "scripts\\index.rhai"
            }"#,
        )
        .expect("write manifest");

        let loader = PluginLoader::new(dir.path().to_path_buf());
        let err = match loader.load_plugin(&plugin_dir) {
            Ok(_) => panic!("backslash entry path must be rejected"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("forward-slash relative paths"));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn load_plugin_rejects_non_rhai_entry() {
        let dir = tempdir().expect("tempdir");
        let plugin_dir = dir.path().join("bad-plugin");
        fs::create_dir(&plugin_dir).expect("mkdir plugin");
        fs::write(
            plugin_dir.join("manifest.json"),
            r#"{
                "id": "bad-plugin",
                "name": "Bad Plugin",
                "version": "1.0.0",
                "entry": "index.txt"
            }"#,
        )
        .expect("write manifest");
        fs::write(plugin_dir.join("index.txt"), "fn hello() { \"bad\" }").expect("write entry");

        let loader = PluginLoader::new(dir.path().to_path_buf());
        let err = match loader.load_plugin(&plugin_dir) {
            Ok(_) => panic!("non-rhai entry must be rejected"),
            Err(err) => err,
        };

        assert!(err.to_string().contains(".rhai"));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn load_plugin_allows_nested_rhai_entry() {
        let dir = tempdir().expect("tempdir");
        let plugin_dir = dir.path().join("nested-plugin");
        let script_dir = plugin_dir.join("scripts");
        fs::create_dir_all(&script_dir).expect("mkdir scripts");
        fs::write(
            plugin_dir.join("manifest.json"),
            r#"{
                "id": "nested-plugin",
                "name": "Nested Plugin",
                "version": "1.0.0",
                "entry": "scripts/index.rhai"
            }"#,
        )
        .expect("write manifest");
        fs::write(script_dir.join("index.rhai"), "fn hello() { \"ok\" }").expect("write entry");

        let loader = PluginLoader::new(dir.path().to_path_buf());
        let plugin = loader.load_plugin(&plugin_dir).expect("load plugin");

        assert_eq!(plugin.manifest().id, "nested-plugin");
    }
}
