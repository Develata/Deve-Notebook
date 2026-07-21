//! plan_ref:
//!   - 15_settings#configuration-settings
//!   - 07_network#static-peer-config
//!
//! Static runtime config validation.

use super::Config;
use anyhow::{Result, bail};
use std::collections::HashSet;

pub(super) fn validate(config: &Config) -> Result<()> {
    validate_runtime_numbers(config)?;
    validate_repo_creation_projection_base(config)?;
    validate_p2p(config)
}

fn validate_repo_creation_projection_base(config: &Config) -> Result<()> {
    if let Some(path) = config.repo_creation_projection_base.as_deref()
        && !path.is_absolute()
    {
        bail!("repo_creation_projection_base must be an absolute path");
    }
    Ok(())
}

fn validate_runtime_numbers(config: &Config) -> Result<()> {
    if config.p2p.connect_interval_ms == 0 {
        bail!("p2p.connect_interval_ms must be greater than 0");
    }
    Ok(())
}

fn validate_p2p(config: &Config) -> Result<()> {
    if let Some(env_name) = config.p2p.inbound_token_env.as_deref() {
        validate_env_name("p2p.inbound_token_env", env_name)?;
    }
    let mut peer_keys = HashSet::new();
    for (index, peer) in config.p2p.peers.iter().enumerate() {
        let prefix = format!("p2p.peers[{index}]");
        validate_peer_id_identity(&format!("{prefix}.peer_id"), &peer.peer_id)?;
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

fn validate_peer_id_identity(key: &str, value: &str) -> Result<()> {
    validate_non_empty(key, value)?;
    let is_canonical_identity = value.len() == 12
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !is_canonical_identity {
        bail!(
            "{key} must be a canonical identity peer id: 12 lowercase hex characters from the peer startup log"
        );
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
    let rest = if let Some(rest) = value.strip_prefix("ws://") {
        rest
    } else if let Some(rest) = value.strip_prefix("wss://") {
        rest
    } else {
        bail!("{key} must use ws:// or wss://");
    };
    if value
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        bail!("{key} must not contain whitespace or control characters");
    }
    if rest.contains('#') {
        bail!("{key} must not include a fragment");
    }
    let authority_end = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() {
        bail!("{key} must include a URL authority");
    }
    if authority.contains('@') {
        bail!("{key} must not include userinfo");
    }
    validate_ws_authority(key, authority)?;
    Ok(())
}

fn validate_ws_authority(key: &str, authority: &str) -> Result<()> {
    if let Some(rest) = authority.strip_prefix('[') {
        let Some((host, suffix)) = rest.split_once(']') else {
            bail!("{key} has invalid IPv6 authority");
        };
        if host.is_empty() {
            bail!("{key} host must be non-empty");
        }
        if let Some(port) = suffix.strip_prefix(':') {
            validate_port(key, port)?;
        } else if !suffix.is_empty() {
            bail!("{key} has invalid authority");
        }
        return Ok(());
    }

    let (host, port) = authority
        .rsplit_once(':')
        .filter(|(host, _)| !host.contains(':'))
        .map_or((authority, None), |(host, port)| (host, Some(port)));
    if host.is_empty() {
        bail!("{key} host must be non-empty");
    }
    if let Some(port) = port {
        validate_port(key, port)?;
    }
    Ok(())
}

fn validate_port(key: &str, port: &str) -> Result<()> {
    match port.parse::<u16>() {
        Ok(port) if port > 0 => Ok(()),
        _ => bail!("{key} port must be a non-zero TCP port"),
    }
}

#[cfg(test)]
mod tests {
    use super::{validate, validate_ws_url};

    #[test]
    fn repo_creation_projection_base_must_be_absolute_when_present() {
        let config = crate::config::Config {
            repo_creation_projection_base: Some("relative/notes".into()),
            ..crate::config::Config::default()
        };
        let error = validate(&config).expect_err("relative first-repo base must fail closed");
        assert!(
            error
                .to_string()
                .contains("repo_creation_projection_base must be an absolute path")
        );

        let config = crate::config::Config {
            repo_creation_projection_base: Some(std::env::temp_dir()),
            ..crate::config::Config::default()
        };
        validate(&config).expect("absolute first-repo base should be accepted");
    }

    #[test]
    fn p2p_ws_url_requires_structured_authority() {
        for value in [
            "ws://",
            "ws:///ws",
            "ws://:3001/ws",
            "ws://peer-b:0/ws",
            "ws://peer-b:bad/ws",
            "ws://user:pass@peer-b:3001/ws",
            "ws://peer-b:3001/ws#fragment",
            "ws://peer b:3001/ws",
        ] {
            validate_ws_url("p2p.peers[0].ws_url", value)
                .expect_err("invalid ws_url must fail closed");
        }
    }

    #[test]
    fn p2p_ws_url_accepts_static_peer_endpoint_shapes() {
        for value in [
            "ws://peer-b:3001/ws",
            "wss://mesh.example.test/ws",
            "ws://[::1]:3001/ws",
            "ws://peer-b:3001/ws?role=fullpeer",
        ] {
            validate_ws_url("p2p.peers[0].ws_url", value).expect("valid ws_url should be accepted");
        }
    }
}
