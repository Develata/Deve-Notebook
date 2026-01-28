// crates\core\src\plugin
//! # Plugin Manifest & Capabilities (插件清单与能力)
//!
//! **架构作用**:
//! 定义插件的元数据（ID, Version）与安全能力清单（Capabilities）。
//!
//! **核心功能清单**:
//! - `PluginManifest`: 插件配置结构体。
//! - `Capability`: 插件请求的权限集合（网络、文件读写、环境变量）。
//! - `check_*`: 权限校验逻辑（Default Deny）。
//!
//! **类型**: Core MUST (核心必选)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub entry: String, // Entry point script (e.g., "main.rhai")
    #[serde(default)]
    pub capabilities: Capability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Capability {
    #[serde(default)]
    pub allow_net: Vec<String>,
    #[serde(default)]
    pub allow_fs_read: Vec<PathBuf>,
    #[serde(default)]
    pub allow_fs_write: Vec<PathBuf>,
    #[serde(default)]
    pub allow_env: Vec<String>,
    #[serde(default)]
    pub allow_source_control: bool,
}

impl Capability {
    /// Normalize path manually (resolve `..` and `.`) to prevent path traversal
    fn normalize_path(path: &Path) -> PathBuf {
        let components = path.components().peekable();
        let mut ret = PathBuf::new();

        for component in components {
            match component {
                std::path::Component::Prefix(..) => ret.push(component.as_os_str()),
                std::path::Component::RootDir => ret.push(component.as_os_str()),
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    ret.pop();
                }
                std::path::Component::Normal(c) => ret.push(c),
            }
        }
        ret
    }

    /// Check if a network domain matches the allow list.
    /// Supports exact match for now.
    pub fn check_net(&self, domain: &str) -> bool {
        self.allow_net.iter().any(|d| d == domain)
    }

    /// Check if an environment variable is allowed.
    pub fn check_env(&self, key: &str) -> bool {
        self.allow_env.iter().any(|k| k == key)
    }

    /// Check if a path is allowed for reading.
    /// Manually normalizes the path to prevent traversal.
    pub fn check_read(&self, path: &Path) -> bool {
        let path = Self::normalize_path(path);
        self.allow_fs_read
            .iter()
            .any(|prefix| path.starts_with(prefix))
    }

    /// Check if a path is allowed for writing.
    /// Manually normalizes the path to prevent traversal.
    pub fn check_write(&self, path: &Path) -> bool {
        let path = Self::normalize_path(path);
        self.allow_fs_write
            .iter()
            .any(|prefix| path.starts_with(prefix))
    }

    /// Check if source control access is allowed.
    pub fn check_source_control(&self) -> bool {
        self.allow_source_control
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_check_net() {
        let cap = Capability {
            allow_net: vec!["api.github.com".to_string(), "google.com".to_string()],
            ..Default::default()
        };

        assert!(cap.check_net("api.github.com"));
        assert!(!cap.check_net("evil.com"));
    }

    #[test]
    fn test_capability_check_fs() {
        let cap = Capability {
            allow_fs_read: vec![PathBuf::from("/data/vault"), PathBuf::from("C:\\Notes")],
            allow_fs_write: vec![PathBuf::from("/data/vault/public")],
            ..Default::default()
        };

        // Read checks
        assert!(cap.check_read(Path::new("/data/vault/notes.md")));
        assert!(cap.check_read(Path::new("C:\\Notes\\file.txt")));
        assert!(!cap.check_read(Path::new("/etc/passwd")));

        // Write checks
        assert!(cap.check_write(Path::new("/data/vault/public/log.txt")));
        assert!(!cap.check_write(Path::new("/data/vault/private.md"))); // Write is more restrictive

        // Path Traversal check
        assert!(!cap.check_read(Path::new("/data/vault/../etc/passwd")));
        assert!(!cap.check_write(Path::new("/data/vault/public/../../private.md")));
    }

    #[test]
    fn test_capability_check_env() {
        let cap = Capability {
            allow_env: vec!["GITHUB_TOKEN".to_string()],
            ..Default::default()
        };

        assert!(cap.check_env("GITHUB_TOKEN"));
        assert!(!cap.check_env("SECRET_KEY"));
    }
}
