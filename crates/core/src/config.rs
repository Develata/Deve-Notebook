// crates\core\src
//! # 核心配置模块 (Core Configuration)
//! plan_ref:
//!   - 13_settings#configuration-settings
//!   - 10_ai_agent#trusted-agent-bridge
//!
//! **架构作用**:
//! 本模块负责处理应用的所有运行时配置，包括环境变量加载和默认值回退。
//! 遵循 12-Factor App 原则，优先从环境变量加载配置。
//!
//! **核心功能清单**:
//! - `AppProfile`: 应用运行模式枚举 (Standard/LowSpec)
//! - `SyncMode`: P2P 同步模式枚举 (Auto/Manual)
//! - `MergeStrategy`: 合并冲突策略枚举 (Manual/Auto)
//! - `Config`: 聚合所有配置项的结构体
//! - `Config::load()`: 从环境加载配置的工厂方法
//!
//! **类型**: Core MUST (核心必选)

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[path = "config/defaults.rs"]
mod defaults;
#[path = "config/env_alias.rs"]
mod env_alias;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBridgeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default = "defaults::agent_bridge_timeout_ms")]
    pub timeout_ms: u64,
}

impl Default for AgentBridgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            trusted: false,
            timeout_ms: defaults::agent_bridge_timeout_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "defaults::ai_mode")]
    pub mode: String,
    #[serde(default = "defaults::ai_native_enabled")]
    pub native_enabled: bool,
    #[serde(default)]
    pub agent_bridge: AgentBridgeConfig,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            mode: defaults::ai_mode(),
            native_enabled: defaults::ai_native_enabled(),
            agent_bridge: AgentBridgeConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "defaults::ui_locale")]
    pub locale: String,
    #[serde(default = "defaults::ui_theme")]
    pub theme: String,
    #[serde(default = "defaults::true_value")]
    pub sidebar_visible: bool,
    #[serde(default = "defaults::true_value")]
    pub statusbar_visible: bool,
    #[serde(default = "defaults::true_value")]
    pub outline_visible: bool,
    #[serde(default = "defaults::outline_width")]
    pub outline_width: usize,
    #[serde(default = "defaults::sidebar_width")]
    pub sidebar_width: usize,
    #[serde(default = "defaults::right_panel_width")]
    pub right_panel_width: usize,
    #[serde(default = "defaults::outer_gutter")]
    pub outer_gutter: usize,
    #[serde(default = "defaults::recent_commands_count")]
    pub recent_commands_count: usize,
    #[serde(default = "defaults::recent_docs_count")]
    pub recent_docs_count: usize,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            locale: defaults::ui_locale(),
            theme: defaults::ui_theme(),
            sidebar_visible: defaults::true_value(),
            statusbar_visible: defaults::true_value(),
            outline_visible: defaults::true_value(),
            outline_width: defaults::outline_width(),
            sidebar_width: defaults::sidebar_width(),
            right_panel_width: defaults::right_panel_width(),
            outer_gutter: defaults::outer_gutter(),
            recent_commands_count: defaults::recent_commands_count(),
            recent_docs_count: defaults::recent_docs_count(),
        }
    }
}

/// 同步模式
/// 控制 P2P 同步的自动化程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    /// 自动模式：后台自动拉取与合并（无冲突时）
    #[default]
    Auto,
    /// 手动模式：接收 payload 后暂存，Merge 必须显式确认
    Manual,
}

impl FromStr for SyncMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("auto") {
            Ok(SyncMode::Auto)
        } else if s.eq_ignore_ascii_case("manual") {
            Ok(SyncMode::Manual)
        } else {
            Err("invalid sync mode")
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
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("standard") {
            Ok(AppProfile::Standard)
        } else if s.eq_ignore_ascii_case("low-spec") {
            Ok(AppProfile::LowSpec)
        } else {
            Err("invalid application profile")
        }
    }
}

/// 合并冲突处理策略 (07_diff_logic.md)
/// 控制 3-Way Merge 时冲突的处理方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    /// 手动模式 (默认): 总是弹出 Diff View 供用户确认
    #[default]
    Manual,
    /// 自动模式 (CRDT 优先): 仅在检测到结构冲突时才弹出
    Auto,
}

impl FromStr for MergeStrategy {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("manual") {
            Ok(MergeStrategy::Manual)
        } else if s.eq_ignore_ascii_case("auto") {
            Ok(MergeStrategy::Auto)
        } else {
            Err("invalid merge strategy")
        }
    }
}

/// 核心配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 当前运行模式
    #[serde(default = "defaults::profile")]
    pub profile: AppProfile,

    // --- 路径配置 ---
    /// 账本目录路径 (Default: "ledger")
    #[serde(default = "defaults::ledger")]
    pub ledger_dir: String,
    /// Vault 根目录 (Default: "vault")
    #[serde(default = "defaults::vault")]
    pub vault_path: String,

    // --- P2P 同步配置 ---
    /// 同步模式 (Auto/Manual)
    #[serde(default)]
    pub sync_mode: SyncMode,

    // --- Diff/Merge 配置 ---
    /// 合并策略: Manual (总是确认) | Auto (CRDT 优先)
    #[serde(default)]
    pub merge_strategy: MergeStrategy,

    // --- 性能调优 ---
    /// 快照保留深度
    #[serde(default = "defaults::snapshot_depth")]
    pub snapshot_depth: usize,
    /// 后台压缩并发度
    #[serde(default = "defaults::concurrency")]
    pub concurrency: usize,
    /// UI defaults and harmless browser preference seeds.
    #[serde(default)]
    pub ui: UiConfig,
    /// AI backend settings.
    #[serde(default)]
    pub ai: AiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile: defaults::profile(),
            ledger_dir: defaults::ledger(),
            vault_path: defaults::vault(),
            sync_mode: SyncMode::default(),
            merge_strategy: MergeStrategy::default(),
            snapshot_depth: defaults::snapshot_depth(),
            concurrency: defaults::concurrency(),
            ui: UiConfig::default(),
            ai: AiConfig::default(),
        }
    }
}

impl Config {
    /// 严格加载配置 (Env > .env > config.toml > Default)。
    ///
    /// Invariants:
    /// - 生产入口遇到坏配置时必须 fail-closed。
    /// - 只有显式宽松调用方才允许回退到默认配置。
    pub fn load_checked() -> anyhow::Result<Self> {
        if let Err(e) = dotenvy::dotenv() {
            tracing::debug!(".env file not found or invalid: {}", e);
        }

        let settings = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(
                config::Environment::with_prefix("DEVE")
                    .prefix_separator("_")
                    .separator("__"),
            )
            .build()
            .context("Failed to build configuration")?;

        let mut config = settings
            .try_deserialize::<Self>()
            .context("Failed to parse configuration")?;
        env_alias::apply_ai_aliases(&mut config)?;
        Ok(config)
    }

    /// 加载配置 (Env > .env > config.toml > Default)
    pub fn load() -> Self {
        Self::load_checked().unwrap_or_else(|e| {
            tracing::warn!("Failed to parse config, using defaults: {}", e);
            Config::default()
        })
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
