// crates\core\src\plugin
//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
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
use crate::plugin::resource_budget::{
    MAX_PLUGIN_COUNT, MAX_PLUGIN_MANIFEST_BYTES, MAX_PLUGIN_SCRIPT_BYTES, read_utf8_file_bounded,
};
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

        let mut plugin_paths = Vec::new();
        for entry in fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type()?.is_dir() {
                plugin_paths.push(path);
                if plugin_paths.len() > MAX_PLUGIN_COUNT {
                    bail!("Plugin directory exceeds the configured plugin-count budget");
                }
            }
        }
        plugin_paths.sort();
        for path in plugin_paths {
            match self
                .load_plugin(&path)
                .with_context(|| format!("Failed to load plugin at {:?}", path))
            {
                Ok(runtime) => {
                    println!("Prepared plugin: {}", runtime.manifest().name);
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

        Ok(plugins)
    }

    pub fn load_plugin(&self, path: &Path) -> Result<Box<dyn PluginRuntime>> {
        // 1. Read manifest.json
        let manifest_path = path.join("manifest.json");
        let manifest_content =
            read_utf8_file_bounded(&manifest_path, MAX_PLUGIN_MANIFEST_BYTES, "plugin manifest")
                .with_context(|| format!("Failed to read manifest.json in {:?}", path))?;

        let manifest: PluginManifest = serde_json::from_str(&manifest_content)
            .with_context(|| "Failed to parse manifest.json")?;

        // 2. Read entry script
        let entry_path = validate_plugin_entry(path, &manifest.entry)?;
        let script_content =
            read_utf8_file_bounded(&entry_path, MAX_PLUGIN_SCRIPT_BYTES, "plugin entry script")
                .with_context(|| {
                    format!(
                        "Failed to read entry script '{}' in {:?}",
                        manifest.entry, path
                    )
                })?;

        // 3. Prepare Runtime. Host APIs are not executable until the composition root activates it.
        let mut runtime = RhaiRuntime::new(manifest.clone(), path.to_path_buf());
        runtime.prepare(manifest, &script_content)?;

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
    if looks_like_windows_drive_path(entry) {
        bail!(
            "Invalid plugin entry '{}': drive prefixes are not allowed",
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
    let canonical_root = fs::canonicalize(plugin_root)
        .with_context(|| format!("Failed to canonicalize plugin directory {:?}", plugin_root))?;
    let canonical_entry = match fs::canonicalize(&resolved) {
        Ok(path) => path,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(resolved),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("Failed to canonicalize plugin entry {:?}", resolved));
        }
    };
    if !canonical_entry.starts_with(&canonical_root) {
        bail!(
            "Invalid plugin entry '{}': resolved entry must stay inside plugin directory",
            entry
        );
    }
    Ok(canonical_entry)
}

#[cfg(not(target_arch = "wasm32"))]
fn looks_like_windows_drive_path(path: &str) -> bool {
    matches!(
        path.as_bytes(),
        [drive, b':', ..] if drive.is_ascii_alphabetic()
    )
}

#[cfg(test)]
mod tests;
