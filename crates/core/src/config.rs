// crates\core\src
//! # 核心配置模块 (Core Configuration)
//! plan_ref:
//!   - 15_settings#configuration-settings
//!   - 16_ai_agent#trusted-agent-bridge
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

use anyhow::{Context, bail};
mod defaults;
mod env_alias;
mod profile;
mod schema;

pub use schema::{
    AgentBridgeConfig, AiConfig, AppProfile, Config, GitBridgeMode, MergeStrategy, P2pConfig,
    P2pPeerConfig, SourceControlConfig, SyncMode, UiConfig,
};

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
        if std::env::var_os("DEVE_VAULT_PATH").is_some() {
            bail!(
                "DEVE_VAULT_PATH is no longer supported; configure repo Projection Locators instead"
            );
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
        if settings.get::<config::Value>("vault_path").is_ok() {
            bail!("vault_path is no longer supported; configure repo Projection Locators instead");
        }

        let mut config = settings
            .clone()
            .try_deserialize::<Self>()
            .context("Failed to parse configuration")?;
        profile::apply_profile_presets(&settings, &mut config);
        env_alias::apply_env_aliases(&mut config)?;
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
mod tests;
