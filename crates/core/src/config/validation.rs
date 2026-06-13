//! plan_ref:
//!   - 15_settings#configuration-settings
//!   - 07_network#static-peer-config
//!
//! Static runtime config validation.

use super::Config;
use anyhow::{Result, bail};
use std::collections::HashSet;

pub(super) fn validate(config: &Config) -> Result<()> {
    validate_p2p(config)
}

fn validate_p2p(config: &Config) -> Result<()> {
    if let Some(env_name) = config.p2p.inbound_token_env.as_deref() {
        validate_env_name("p2p.inbound_token_env", env_name)?;
    }
    let mut peer_keys = HashSet::new();
    for (index, peer) in config.p2p.peers.iter().enumerate() {
        let prefix = format!("p2p.peers[{index}]");
        validate_non_empty(&format!("{prefix}.peer_id"), &peer.peer_id)?;
        validate_repo_id(&format!("{prefix}.repo_id"), &peer.repo_id)?;
        validate_ws_url(&format!("{prefix}.ws_url"), &peer.ws_url)?;
        validate_env_name(&format!("{prefix}.auth_token_env"), &peer.auth_token_env)?;
        let repo_id = uuid::Uuid::parse_str(&peer.repo_id)
            .expect("p2p repo_id was validated immediately above");
        let peer_key = (peer.peer_id.clone(), repo_id, peer.ws_url.clone());
        if !peer_keys.insert(peer_key) {
            bail!("{prefix} duplicates peer identity tuple peer_id + repo_id + ws_url");
        }
    }
    Ok(())
}

fn validate_non_empty(key: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{key} must be non-empty");
    }
    if value != value.trim() {
        bail!("{key} must not contain outer whitespace");
    }
    Ok(())
}

fn validate_env_name(key: &str, value: &str) -> Result<()> {
    validate_non_empty(key, value)?;
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        bail!("{key} must be non-empty");
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        bail!("{key} must be a valid environment variable name");
    }
    if !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
        bail!("{key} must be a valid environment variable name");
    }
    Ok(())
}

fn validate_repo_id(key: &str, value: &str) -> Result<()> {
    validate_non_empty(key, value)?;
    uuid::Uuid::parse_str(value).map_err(|_| anyhow::anyhow!("{key} must be a UUID"))?;
    Ok(())
}

fn validate_ws_url(key: &str, value: &str) -> Result<()> {
    validate_non_empty(key, value)?;
    if !(value.starts_with("ws://") || value.starts_with("wss://")) {
        bail!("{key} must use ws:// or wss://");
    }
    Ok(())
}
