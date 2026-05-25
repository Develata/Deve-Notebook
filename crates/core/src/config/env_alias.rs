//! plan_ref:
//!   - 15_settings#configuration-settings
//!   - 16_ai_agent#trusted-agent-bridge
//!
//! Compatibility environment aliases for runtime config.

use anyhow::anyhow;
use std::env::VarError;

use super::Config;

pub(super) fn apply_env_aliases(config: &mut Config) -> anyhow::Result<()> {
    if let Some(value) = env_bool("DEVE_AI_AGENT_BRIDGE_ENABLED")? {
        config.ai.agent_bridge.enabled = value;
    }
    if let Some(value) = env_bool("DEVE_AI_AGENT_BRIDGE_TRUSTED")? {
        config.ai.agent_bridge.trusted = value;
    }
    if let Some(value) = env_usize("MEM_CACHE_MB")? {
        config.mem_cache_mb = value;
    }
    Ok(())
}

fn env_bool(key: &str) -> anyhow::Result<Option<bool>> {
    match std::env::var(key) {
        Ok(value) => parse_env_bool(key, &value).map(Some),
        Err(VarError::NotPresent) => Ok(None),
        Err(err) => Err(anyhow!("Failed to read environment variable {key}: {err}")),
    }
}

fn env_usize(key: &str) -> anyhow::Result<Option<usize>> {
    match std::env::var(key) {
        Ok(value) => parse_env_usize(key, &value).map(Some),
        Err(VarError::NotPresent) => Ok(None),
        Err(err) => Err(anyhow!("Failed to read environment variable {key}: {err}")),
    }
}

fn parse_env_bool(key: &str, value: &str) -> anyhow::Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(anyhow!(
            "Invalid boolean environment variable {key}: {value}"
        )),
    }
}

fn parse_env_usize(key: &str, value: &str) -> anyhow::Result<usize> {
    value
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow!("Invalid integer environment variable {key}: {value}"))
}
