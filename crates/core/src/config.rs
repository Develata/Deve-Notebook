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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
pub struct Config {
    /// 当前运行模式
    pub profile: AppProfile,
    
    // --- 路径配置 ---
    /// 账本目录路径 (Default: "ledger")
    /// Contains local.redb and remotes/ subdirectory
    pub ledger_dir: String,
    /// Vault 根目录 (Default: "vault")
    pub vault_path: String,

    // --- P2P 同步配置 ---
    /// 同步模式 (Auto/Manual) - Env: DEVE_SYNC_MODE
    pub sync_mode: SyncMode,

    // --- 性能调优 ---
    /// 快照保留深度 (Standard: 100, LowSpec: 10)
    pub snapshot_depth: usize,
    /// 后台压缩并发度 (Standard: 4, LowSpec: 1)
    pub concurrency: usize,
}

impl Config {
    /// 从环境变量加载配置
    pub fn load() -> Self {
        // 1. Load Profile first to determine defaults
        let profile_str = env::var("DEVE_PROFILE").unwrap_or_else(|_| "standard".to_string());
        let profile = AppProfile::from_str(&profile_str).unwrap_or(AppProfile::Standard);

        // 2. Load path configuration
        let ledger_dir = env::var("DEVE_LEDGER_DIR").unwrap_or_else(|_| "ledger".to_string());
        let vault_path = env::var("DEVE_VAULT_PATH").unwrap_or_else(|_| "vault".to_string());

        // 3. Load P2P sync configuration
        let sync_mode_str = env::var("DEVE_SYNC_MODE").unwrap_or_else(|_| "auto".to_string());
        let sync_mode = SyncMode::from_str(&sync_mode_str).unwrap_or_default();

        // 4. Load performance tuning with profile-based defaults
        let snapshot_depth = env::var("DEVE_SNAPSHOT_DEPTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| match profile {
                AppProfile::Standard => 100,
                AppProfile::LowSpec => 10,
            });

        let concurrency = env::var("DEVE_CONCURRENCY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| match profile {
                AppProfile::Standard => 4,
                AppProfile::LowSpec => 1,
            });

        Self {
            profile,
            ledger_dir,
            vault_path,
            sync_mode,
            snapshot_depth,
            concurrency,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::load();
        // Assuming no env vars set in test env
        assert_eq!(config.profile, AppProfile::Standard);
        assert_eq!(config.snapshot_depth, 100);
    }
}
