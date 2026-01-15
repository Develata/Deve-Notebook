//! # 核心配置模块 (Core Configuration)
//!
//! **架构作用**:
//! 本模块负责处理应用的所有运行时配置，包括环境变量加载和默认值回退。
//! 遵循 12-Factor App 原则，优先从环境变量加载配置。
//!
//! **核心功能清单**:
//! - `AppProfile`: 应用运行模式枚举 (Standard/LowSpec)
//! - `SyncMode`: 同步模式枚举 (Auto/Manual)
//! - `Config`: 聚合所有配置项的结构体
//! - `Config::load()`: 从环境加载配置的工厂方法
//!
//! **类型**: Core MUST (核心必选)

use std::str::FromStr;
use std::env;
use serde::{Serialize, Deserialize};

/// 同步模式
/// 控制 P2P 同步的自动化程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    /// 自动模式：后台自动拉取与合并（无冲突时）
    #[default]
    Auto,
    /// 手动模式：仅交换 Vector，Fetch/Merge 必须显式触发
    Manual,
}

impl FromStr for SyncMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "manual" | "strict" | "strictmode" => Ok(SyncMode::Manual),
            _ => Ok(SyncMode::Auto), // Default to Auto
        }
    }
}

/// 应用运行模式预设
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppProfile {
    /// 标准模式 (1GB+ RAM)：启用全功能 (SSR, Search, Graph)
    Standard,
    /// 低配模式 (512MB RAM)：禁用重型功能，降低并发
    LowSpec,
}

impl FromStr for AppProfile {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low-spec" | "lowspec" | "low" => Ok(AppProfile::LowSpec),
            _ => Ok(AppProfile::Standard), // Default to Standard for any other value
        }
    }
}

/// 核心配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 当前运行模式
    #[serde(default = "default_profile")]
    pub profile: AppProfile,
    
    // --- 路径配置 ---
    /// 账本目录路径 (Default: "ledger")
    #[serde(default = "default_ledger")]
    pub ledger_dir: String,
    /// Vault 根目录 (Default: "vault")
    #[serde(default = "default_vault")]
    pub vault_path: String,

    // --- P2P 同步配置 ---
    /// 同步模式 (Auto/Manual)
    #[serde(default)]
    pub sync_mode: SyncMode,

    // --- 性能调优 ---
    /// 快照保留深度
    #[serde(default = "default_snapshot_depth")]
    pub snapshot_depth: usize,
    /// 后台压缩并发度
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

fn default_profile() -> AppProfile { AppProfile::Standard }
fn default_ledger() -> String { "ledger".to_string() }
fn default_vault() -> String { "vault".to_string() }
fn default_snapshot_depth() -> usize { 100 }
fn default_concurrency() -> usize { 4 }

impl Config {
    /// 加载配置 (Env > .env > config.toml > Default)
    pub fn load() -> Self {
        // 1. Load .env file if present
        if let Err(e) = dotenvy::dotenv() {
            tracing::debug!(".env file not found or invalid: {}", e);
        }

        // 2. Build Config Source
        // Layering: Defaults -> File(config.toml) -> Env(DEVE_*)
        let settings = config::Config::builder()
            // Default Fallbacks implemented via Serde defaults, so we just build empty source initially
            // Add config file (optional)
            .add_source(config::File::with_name("config").required(false))
            // Add environment variables (prefix DEVE_)
            // e.g. DEVE_LEDGER_DIR -> ledger_dir
            .add_source(config::Environment::with_prefix("DEVE").separator("_"))
            .build()
            .expect("Failed to build configuration");

        // 3. Deserialize
        settings.try_deserialize::<Self>().unwrap_or_else(|e| {
            tracing::warn!("Failed to parse config, using defaults: {}", e);
            // Fallback to manual construction if partial parsing fails heavily
            Config {
                profile: default_profile(),
                ledger_dir: default_ledger(),
                vault_path: default_vault(),
                sync_mode: SyncMode::default(),
                snapshot_depth: default_snapshot_depth(),
                concurrency: default_concurrency(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        // Just verify it builds without panic
        let config = Config::load();
        assert!(!config.ledger_dir.is_empty());
    }
}
