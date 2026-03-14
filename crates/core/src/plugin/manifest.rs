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
    #[serde(default)]
    pub allow_search: bool,
    #[serde(default)]
    pub allow_skill: bool,
    #[serde(default)]
    pub allow_mcp: bool,
    #[serde(default)]
    pub allow_project_tree: bool,
}

impl Capability {
    /// 词法归一化路径，显式兼容 Windows `\` 与 Unix `/`。
    ///
    /// Invariants:
    /// - 仅做词法层 `.` / `..` 规约，不访问文件系统。
    /// - `C:\Notes\file.md` 与 `C:/Notes/file.md` 必须收敛到同一前缀语义。
    fn normalize_path(path: &Path) -> String {
        let raw = path.to_string_lossy().replace('\\', "/");
        let (prefix, absolute, rest) = match raw.as_bytes() {
            [drive, b':', b'/', ..] if drive.is_ascii_alphabetic() => {
                (raw[..2].to_string(), true, &raw[3..])
            }
            [drive, b':'] if drive.is_ascii_alphabetic() => (raw[..2].to_string(), false, ""),
            [drive, b':', ..] if drive.is_ascii_alphabetic() => {
                (raw[..2].to_string(), false, &raw[2..])
            }
            _ if raw.starts_with('/') => (String::new(), true, raw.trim_start_matches('/')),
            _ => (String::new(), false, raw.as_str()),
        };
        let mut parts = Vec::new();
        for part in rest.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                other => parts.push(other),
            }
        }
        let joined = parts.join("/");
        match (prefix.is_empty(), absolute, joined.is_empty()) {
            (false, true, true) => format!("{prefix}/"),
            (false, true, false) => format!("{prefix}/{joined}"),
            (false, false, true) => prefix,
            (false, false, false) => format!("{prefix}/{joined}"),
            (true, true, true) => "/".to_string(),
            (true, true, false) => format!("/{joined}"),
            (true, false, _) => joined,
        }
    }

    fn has_allowed_prefix(path: &str, prefix: &str) -> bool {
        path == prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'))
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
            .map(|prefix| Self::normalize_path(prefix))
            .any(|prefix| Self::has_allowed_prefix(&path, &prefix))
    }

    /// Check if a path is allowed for writing.
    /// Manually normalizes the path to prevent traversal.
    pub fn check_write(&self, path: &Path) -> bool {
        let path = Self::normalize_path(path);
        self.allow_fs_write
            .iter()
            .map(|prefix| Self::normalize_path(prefix))
            .any(|prefix| Self::has_allowed_prefix(&path, &prefix))
    }

    /// Check if source control access is allowed.
    pub fn check_source_control(&self) -> bool {
        self.allow_source_control
    }

    /// Check if search access is allowed. (检查搜索权限)
    pub fn check_search(&self) -> bool {
        self.allow_search
    }

    /// Check if skill access is allowed. (检查技能权限)
    pub fn check_skill(&self) -> bool {
        self.allow_skill
    }

    /// Check if MCP access is allowed. (检查 MCP 权限)
    pub fn check_mcp(&self) -> bool {
        self.allow_mcp
    }

    /// Check if project tree access is allowed. (检查项目树权限)
    pub fn check_project_tree(&self) -> bool {
        self.allow_project_tree
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
    fn test_capability_does_not_match_sibling_prefixes() {
        let cap = Capability {
            allow_fs_read: vec![PathBuf::from("/data/vault"), PathBuf::from("C:\\Notes")],
            ..Default::default()
        };

        assert!(!cap.check_read(Path::new("/data/vaults/notes.md")));
        assert!(!cap.check_read(Path::new("C:\\Notes2\\file.txt")));
    }

    #[test]
    fn test_capability_normalizes_mixed_windows_separators() {
        let cap = Capability {
            allow_fs_read: vec![PathBuf::from("C:\\Notes")],
            allow_fs_write: vec![PathBuf::from("C:/Notes/Public")],
            ..Default::default()
        };

        assert!(cap.check_read(Path::new("C:/Notes/sub/file.md")));
        assert!(cap.check_write(Path::new("C:\\Notes\\Public\\log.txt")));
        assert!(!cap.check_write(Path::new("C:\\Notes\\Private\\log.txt")));
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
