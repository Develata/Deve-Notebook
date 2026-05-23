//! plan_ref:
//!   - 13_settings#configuration-settings
//!
//! CLI-writable `config.toml` key schema.

use anyhow::anyhow;
use toml::Value;

const CONFIG_KEY_SPECS: &[ConfigKeySpec] = &[
    ConfigKeySpec::string("ai.mode", &["native", "trusted-cli"]),
    ConfigKeySpec::bool("ai.native_enabled"),
    ConfigKeySpec::bool("ai.agent_bridge.enabled"),
    ConfigKeySpec::bool("ai.agent_bridge.trusted"),
    ConfigKeySpec::integer("ai.agent_bridge.timeout_ms"),
    ConfigKeySpec::integer("concurrency"),
    ConfigKeySpec::string("ledger_dir", &[]),
    ConfigKeySpec::integer("mem_cache_mb"),
    ConfigKeySpec::string("merge_strategy", &["manual", "auto"]),
    ConfigKeySpec::string("profile", &["standard", "low-spec"]),
    ConfigKeySpec::integer("snapshot_depth"),
    ConfigKeySpec::string("sync_mode", &["auto", "manual"]),
    ConfigKeySpec::string("ui.locale", &["auto", "en-US", "zh-CN"]),
    ConfigKeySpec::integer("ui.outer_gutter"),
    ConfigKeySpec::bool("ui.outline_visible"),
    ConfigKeySpec::integer("ui.outline_width"),
    ConfigKeySpec::integer("ui.recent_commands_count"),
    ConfigKeySpec::integer("ui.recent_docs_count"),
    ConfigKeySpec::integer("ui.right_panel_width"),
    ConfigKeySpec::bool("ui.sidebar_visible"),
    ConfigKeySpec::integer("ui.sidebar_width"),
    ConfigKeySpec::bool("ui.statusbar_visible"),
    ConfigKeySpec::string("ui.theme", &["auto", "light", "dark"]),
];

#[derive(Debug, Clone, Copy)]
struct ConfigKeySpec {
    key: &'static str,
    kind: ValueKind,
}

impl ConfigKeySpec {
    const fn string(key: &'static str, choices: &'static [&'static str]) -> Self {
        Self {
            key,
            kind: ValueKind::String { choices },
        }
    }

    const fn bool(key: &'static str) -> Self {
        Self {
            key,
            kind: ValueKind::Bool,
        }
    }

    const fn integer(key: &'static str) -> Self {
        Self {
            key,
            kind: ValueKind::Integer,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ValueKind {
    String { choices: &'static [&'static str] },
    Bool,
    Integer,
}

#[cfg(test)]
impl ValueKind {
    fn plan_type(self) -> &'static str {
        match self {
            Self::String { .. } => "String",
            Self::Bool => "Bool",
            Self::Integer => "Number",
        }
    }

    fn choices(self) -> &'static [&'static str] {
        match self {
            Self::String { choices } => choices,
            Self::Bool | Self::Integer => &[],
        }
    }
}

pub(super) fn parse_whitelisted_value(key: &str, value: &str) -> anyhow::Result<Value> {
    match value_kind(key)? {
        ValueKind::String { choices } => parse_string(value, choices),
        ValueKind::Bool => parse_bool(value),
        ValueKind::Integer => parse_integer(value),
    }
}

fn value_kind(key: &str) -> anyhow::Result<ValueKind> {
    CONFIG_KEY_SPECS
        .iter()
        .find(|spec| spec.key == key)
        .map(|spec| spec.kind)
        .ok_or_else(|| anyhow!("Unsupported config key: {}", key))
}

fn parse_string(value: &str, choices: &[&str]) -> anyhow::Result<Value> {
    let value = value.trim().trim_matches('"').to_string();
    if !choices.is_empty() && !choices.iter().any(|choice| *choice == value) {
        return Err(anyhow!(
            "Invalid value '{}'; expected one of {:?}",
            value,
            choices
        ));
    }
    Ok(Value::String(value))
}

fn parse_bool(value: &str) -> anyhow::Result<Value> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(Value::Boolean(true)),
        "false" | "0" | "no" | "off" => Ok(Value::Boolean(false)),
        _ => Err(anyhow!("Invalid boolean value: {}", value)),
    }
}

fn parse_integer(value: &str) -> anyhow::Result<Value> {
    let parsed = value
        .trim()
        .parse::<i64>()
        .map_err(|_| anyhow!("Invalid integer value: {}", value))?;
    if parsed < 0 {
        return Err(anyhow!("Integer config values must be non-negative"));
    }
    Ok(Value::Integer(parsed))
}

#[cfg(test)]
pub(crate) fn supported_config_keys() -> impl Iterator<Item = &'static str> {
    CONFIG_KEY_SPECS.iter().map(|spec| spec.key)
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigKeySpecView {
    pub key: &'static str,
    pub plan_type: &'static str,
    pub choices: &'static [&'static str],
}

#[cfg(test)]
pub(crate) fn config_key_specs() -> impl Iterator<Item = ConfigKeySpecView> {
    CONFIG_KEY_SPECS.iter().map(|spec| ConfigKeySpecView {
        key: spec.key,
        plan_type: spec.kind.plan_type(),
        choices: spec.kind.choices(),
    })
}
