//! # 核心配置模块 (Core Configuration)
//!
//! 本模块负责处理应用的所有运行时配置，包括环境变量加载和默认值回退。
//!
//! ## 架构作用
//!
//! * **统一配置源**：所有核心组件（Ledger, Sync, VFS）不得直接读取 `std::env`，必须通过本模块提供的 `Config` 结构体获取参数。
//! * **环境驱动**：遵循 12-Factor App 原则，优先从环境变量加载配置。
//! * **预设管理**：提供标准 (Standard) 和低配 (Low-Spec) 两种预设，适配不同硬件环境。
//!
//! ## 核心功能清单
//!
//! - `AppProfile`: 应用运行模式枚举 (Standard/LowSpec).
//! - `Config`: 聚合所有配置项的结构体.
//! - `Config::load()`: 从环境加载配置的工厂方法.
//!
//! ## 核心必选路径 (Core MUST)
//!
//! 本模块属于 **Core MUST**。系统启动时必须首先初始化配置。
//!

use std::str::FromStr;
use std::env;

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

        // 2. Load other values or apply profile defaults
        let ledger_dir = env::var("DEVE_LEDGER_DIR").unwrap_or_else(|_| "ledger".to_string());
        let vault_path = env::var("DEVE_VAULT_PATH").unwrap_or_else(|_| "vault".to_string());

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
