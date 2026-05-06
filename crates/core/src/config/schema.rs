//! plan_ref:
//!   - 13_settings#configuration-settings
//!   - 10_ai_agent#trusted-agent-bridge
//!
//! Runtime config schema and serde/default contracts.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::defaults;

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

/// 同步模式。
/// 控制 P2P 同步的自动化程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    /// 自动模式：后台自动拉取与合并（无冲突时）。
    #[default]
    Auto,
    /// 手动模式：接收 payload 后暂存，Merge 必须显式确认。
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

/// 应用运行模式预设。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppProfile {
    /// 标准模式：启用完整运行时能力。
    Standard,
    /// 低配模式：禁用重型功能并降低并发。
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

/// 合并冲突处理策略。
/// 控制 3-Way Merge 时冲突的处理方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    /// 手动模式：总是弹出 Diff View 供用户确认。
    #[default]
    Manual,
    /// 自动模式：仅在检测到结构冲突时才弹出。
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

/// 核心配置结构体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 当前运行模式。
    #[serde(default = "defaults::profile")]
    pub profile: AppProfile,
    /// 账本目录路径。
    #[serde(default = "defaults::ledger")]
    pub ledger_dir: String,
    /// Vault 根目录。
    #[serde(default = "defaults::vault")]
    pub vault_path: String,
    /// 同步模式。
    #[serde(default)]
    pub sync_mode: SyncMode,
    /// 合并策略。
    #[serde(default)]
    pub merge_strategy: MergeStrategy,
    /// 快照保留深度。
    #[serde(default = "defaults::snapshot_depth")]
    pub snapshot_depth: usize,
    /// 后台压缩并发度。
    #[serde(default = "defaults::concurrency")]
    pub concurrency: usize,
    /// UI 默认值与无害浏览器偏好种子。
    #[serde(default)]
    pub ui: UiConfig,
    /// AI 后端设置。
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
