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
}

impl Capability {
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
    /// Checks if the requested path starts with any allowed path prefix.
    pub fn check_read(&self, path: &Path) -> bool {
        self.allow_fs_read.iter().any(|prefix| path.starts_with(prefix))
    }

    /// Check if a path is allowed for writing.
    pub fn check_write(&self, path: &Path) -> bool {
        self.allow_fs_write.iter().any(|prefix| path.starts_with(prefix))
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
