//! plan_ref:
//!   - 15_settings#configuration-settings
//!   - 16_ai_agent#trusted-agent-bridge
//!
//! Compatibility environment aliases for runtime config.

use anyhow::anyhow;
use std::env::VarError;

use super::{Config, GitBridgeMode, P2pPeerConfig};

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
    apply_source_control_env_aliases(config)?;
    apply_p2p_env_aliases(config)?;
    Ok(())
}

fn apply_source_control_env_aliases(config: &mut Config) -> anyhow::Result<()> {
    if let Some(value) = env_string_any(&[
        "DEVE_SOURCE_CONTROL__GIT_BRIDGE",
        "DEVE_SOURCE_CONTROL_GIT_BRIDGE",
    ])? {
        config.source_control.git_bridge = value
            .parse::<GitBridgeMode>()
            .map_err(|_| anyhow!("Invalid source_control.git_bridge environment value: {value}"))?;
    }
    Ok(())
}

fn apply_p2p_env_aliases(config: &mut Config) -> anyhow::Result<()> {
    if let Some(value) = env_bool_any(&["DEVE_P2P__ENABLED", "DEVE_P2P_ENABLED"])? {
        config.p2p.enabled = value;
    }
    if let Some(value) = env_u64_any(&[
        "DEVE_P2P__CONNECT_INTERVAL_MS",
        "DEVE_P2P_CONNECT_INTERVAL_MS",
    ])? {
        config.p2p.connect_interval_ms = value;
    }
    if let Some(value) =
        env_string_any(&["DEVE_P2P__INBOUND_TOKEN_ENV", "DEVE_P2P_INBOUND_TOKEN_ENV"])?
    {
        config.p2p.inbound_token_env = Some(value);
    }

    let peers = p2p_peers_from_env()?;
    if !peers.is_empty() {
        config.p2p.peers = peers;
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

fn env_bool_any(keys: &[&str]) -> anyhow::Result<Option<bool>> {
    for key in keys {
        if let Some(value) = env_bool(key)? {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn env_usize(key: &str) -> anyhow::Result<Option<usize>> {
    match std::env::var(key) {
        Ok(value) => parse_env_usize(key, &value).map(Some),
        Err(VarError::NotPresent) => Ok(None),
        Err(err) => Err(anyhow!("Failed to read environment variable {key}: {err}")),
    }
}

fn env_u64_any(keys: &[&str]) -> anyhow::Result<Option<u64>> {
    for key in keys {
        match std::env::var(key) {
            Ok(value) => return parse_env_u64(key, &value).map(Some),
            Err(VarError::NotPresent) => {}
            Err(err) => return Err(anyhow!("Failed to read environment variable {key}: {err}")),
        }
    }
    Ok(None)
}

fn env_string_any(keys: &[&str]) -> anyhow::Result<Option<String>> {
    for key in keys {
        match std::env::var(key) {
            Ok(value) => return Ok(Some(value)),
            Err(VarError::NotPresent) => {}
            Err(err) => return Err(anyhow!("Failed to read environment variable {key}: {err}")),
        }
    }
    Ok(None)
}

fn p2p_peers_from_env() -> anyhow::Result<Vec<P2pPeerConfig>> {
    let mut peers = Vec::new();
    for index in 0..32 {
        let safe_prefix = format!("DEVE_P2P_MESH_PEER_{index}_");
        let nested_prefix = format!("DEVE_P2P__PEERS__{index}__");
        let Some(label) = env_string_any(&[
            &format!("{safe_prefix}LABEL"),
            &format!("{nested_prefix}LABEL"),
        ])?
        else {
            break;
        };
        let peer_id = required_env_string_any(&[
            &format!("{safe_prefix}PEER_ID"),
            &format!("{nested_prefix}PEER_ID"),
        ])?;
        let repo_id = required_env_string_any(&[
            &format!("{safe_prefix}REPO_ID"),
            &format!("{nested_prefix}REPO_ID"),
        ])?;
        let ws_url = required_env_string_any(&[
            &format!("{safe_prefix}WS_URL"),
            &format!("{nested_prefix}WS_URL"),
        ])?;
        let auth_token_env = required_env_string_any(&[
            &format!("{safe_prefix}AUTH_TOKEN_ENV"),
            &format!("{nested_prefix}AUTH_TOKEN_ENV"),
        ])?;
        let enabled = env_bool_any(&[
            &format!("{safe_prefix}ENABLED"),
            &format!("{nested_prefix}ENABLED"),
        ])?
        .unwrap_or(true);
        peers.push(P2pPeerConfig {
            label,
            peer_id,
            repo_id,
            ws_url,
            auth_token_env,
            enabled,
        });
    }
    Ok(peers)
}

fn required_env_string_any(keys: &[&str]) -> anyhow::Result<String> {
    for key in keys {
        match std::env::var(key) {
            Ok(value) => return Ok(value),
            Err(VarError::NotPresent) => {}
            Err(err) => return Err(anyhow!("Failed to read environment variable {key}: {err}")),
        }
    }
    Err(anyhow!(
        "Missing required P2P peer environment; tried {}",
        keys.join(", ")
    ))
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

fn parse_env_u64(key: &str, value: &str) -> anyhow::Result<u64> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| anyhow!("Invalid integer environment variable {key}: {value}"))
}
