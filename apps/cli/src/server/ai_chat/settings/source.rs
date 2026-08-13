//! plan_ref:
//!   - 15_settings#native-ai-provider-settings

use super::{ProviderProtocol, ProviderSettingsSnapshot, SettingsSource};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;

pub(crate) const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
pub(crate) const DEFAULT_MODEL: &str = "gpt-4o-mini";
pub(crate) const DEFAULT_MAX_TOKENS: u32 = 4096;
const ANTHROPIC_DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const MAX_MAX_TOKENS: u32 = 131_072;

pub(super) fn default_snapshot() -> ProviderSettingsSnapshot {
    ProviderSettingsSnapshot {
        provider: ProviderProtocol::default(),
        base_url: DEFAULT_BASE_URL.to_string(),
        api_key: String::new(),
        model: DEFAULT_MODEL.to_string(),
        max_tokens: DEFAULT_MAX_TOKENS,
        revision: 1,
    }
}

pub(super) fn snapshot_from_environment(
    values: &BTreeMap<String, String>,
) -> Result<Option<(ProviderSettingsSnapshot, SettingsSource)>> {
    const CANONICAL: [&str; 5] = [
        "AI_PROVIDER",
        "AI_BASE_URL",
        "AI_API_KEY",
        "AI_MODEL",
        "AI_MAX_TOKENS",
    ];
    let canonical_present = CANONICAL
        .iter()
        .any(|key| nonempty(values.get(*key)).is_some());
    let openai_alias = nonempty(values.get("OPENAI_API_KEY"));
    let anthropic_alias = nonempty(values.get("ANTHROPIC_API_KEY"));
    if !canonical_present && openai_alias.is_none() && anthropic_alias.is_none() {
        return Ok(None);
    }
    if !canonical_present && openai_alias.is_some() && anthropic_alias.is_some() {
        bail!("ambiguous AI provider key aliases");
    }
    let mut snapshot = default_snapshot();
    if !canonical_present && anthropic_alias.is_some() {
        snapshot.provider = ProviderProtocol::AnthropicMessages;
        snapshot.base_url = ANTHROPIC_DEFAULT_BASE_URL.to_string();
        snapshot.model = ANTHROPIC_DEFAULT_MODEL.to_string();
    }
    if let Some(provider) = nonempty(values.get("AI_PROVIDER")) {
        snapshot.provider = serde_json::from_value(serde_json::Value::String(provider.to_string()))
            .context("invalid AI_PROVIDER")?;
    }
    if let Some(value) = nonempty(values.get("AI_BASE_URL")) {
        snapshot.base_url = value.to_string();
    }
    snapshot.api_key = nonempty(values.get("AI_API_KEY"))
        .or(if canonical_present {
            None
        } else {
            openai_alias.or(anthropic_alias)
        })
        .unwrap_or_default()
        .to_string();
    if let Some(value) = nonempty(values.get("AI_MODEL")) {
        snapshot.model = value.to_string();
    }
    if let Some(value) = nonempty(values.get("AI_MAX_TOKENS")) {
        snapshot.max_tokens = value.parse().context("invalid AI_MAX_TOKENS")?;
    }
    validate_snapshot(&snapshot)?;
    Ok(Some((snapshot, SettingsSource::Environment)))
}

fn nonempty(value: Option<&String>) -> Option<&str> {
    value
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn validate_snapshot(snapshot: &ProviderSettingsSnapshot) -> Result<()> {
    if snapshot.base_url.len() > 2048 || snapshot.model.is_empty() || snapshot.model.len() > 256 {
        bail!("AI provider setting length is invalid");
    }
    if snapshot.api_key.len() > 8192
        || snapshot.api_key.chars().any(char::is_control)
        || snapshot.model.chars().any(char::is_control)
    {
        bail!("AI provider setting contains invalid characters");
    }
    if !(1..=MAX_MAX_TOKENS).contains(&snapshot.max_tokens) {
        bail!("AI max tokens is outside the supported range");
    }
    let url = reqwest::Url::parse(&snapshot.base_url).context("invalid AI base URL")?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        bail!("AI base URL contains unsupported components");
    }
    let loopback = url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    });
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        bail!("AI base URL requires HTTPS outside loopback");
    }
    Ok(())
}
