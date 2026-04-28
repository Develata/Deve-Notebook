//! plan_ref:
//!   - 12_commands#cli-commands
//!   - 13_settings#configuration-settings

use anyhow::{Context, anyhow};
use std::path::Path;
use toml::Value;
use toml::value::Table;

const CONFIG_FILE: &str = "config.toml";
#[cfg(test)]
const SUPPORTED_CONFIG_KEYS: &[&str] = &[
    "ai.agent_bridge.enabled",
    "ai.agent_bridge.timeout_ms",
    "ai.agent_bridge.trusted",
    "ai.mode",
    "ai.native_enabled",
    "concurrency",
    "ledger_dir",
    "merge_strategy",
    "profile",
    "snapshot_depth",
    "sync_mode",
    "ui.locale",
    "ui.outer_gutter",
    "ui.outline_visible",
    "ui.outline_width",
    "ui.recent_commands_count",
    "ui.recent_docs_count",
    "ui.right_panel_width",
    "ui.sidebar_visible",
    "ui.sidebar_width",
    "ui.statusbar_visible",
    "ui.theme",
    "vault_path",
];

pub fn print(config: &deve_core::config::Config) -> anyhow::Result<()> {
    let output = toml::to_string_pretty(config).context("Failed to render config")?;
    print!("{output}");
    Ok(())
}

pub fn set(key: &str, value: &str) -> anyhow::Result<()> {
    set_in_file(Path::new(CONFIG_FILE), key, value)
}

fn set_in_file(path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
    let parsed = parse_whitelisted_value(key, value)?;
    let mut root = load_root(path)?;
    let parts = split_key(key)?;

    insert_value(&mut root, &parts, parsed)?;
    let output = toml::to_string_pretty(&Value::Table(root)).context("Failed to render config")?;
    validate_config(&output)?;
    std::fs::write(path, output).with_context(|| format!("Failed to write {:?}", path))?;
    println!("Updated {}: {}", path.display(), key);
    Ok(())
}

fn load_root(path: &Path) -> anyhow::Result<Table> {
    if !path
        .try_exists()
        .with_context(|| format!("Failed to stat {:?}", path))?
    {
        return Ok(Table::new());
    }

    let input =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))?;
    if input.trim().is_empty() {
        return Ok(Table::new());
    }

    match toml::from_str::<Value>(&input).context("Failed to parse config.toml")? {
        Value::Table(table) => Ok(table),
        _ => Err(anyhow!("config.toml root must be a TOML table")),
    }
}

fn split_key(key: &str) -> anyhow::Result<Vec<&str>> {
    let parts: Vec<&str> = key.split('.').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return Err(anyhow!("Config key cannot be empty"));
    }
    Ok(parts)
}

fn insert_value(table: &mut Table, parts: &[&str], value: Value) -> anyhow::Result<()> {
    if parts.len() == 1 {
        table.insert(parts[0].to_string(), value);
        return Ok(());
    }

    let entry = table
        .entry(parts[0].to_string())
        .or_insert_with(|| Value::Table(Table::new()));
    match entry {
        Value::Table(child) => insert_value(child, &parts[1..], value),
        _ => Err(anyhow!(
            "Cannot set {} because {} is already a scalar",
            parts.join("."),
            parts[0]
        )),
    }
}

fn parse_whitelisted_value(key: &str, value: &str) -> anyhow::Result<Value> {
    match value_kind(key)? {
        ValueKind::String { choices } => parse_string(value, choices),
        ValueKind::Bool => parse_bool(value),
        ValueKind::Integer => parse_integer(value),
    }
}

fn value_kind(key: &str) -> anyhow::Result<ValueKind> {
    match key {
        "profile" => Ok(ValueKind::String {
            choices: &["standard", "low-spec"],
        }),
        "sync_mode" => Ok(ValueKind::String {
            choices: &["auto", "manual"],
        }),
        "merge_strategy" => Ok(ValueKind::String {
            choices: &["manual", "auto"],
        }),
        "ui.locale" => Ok(ValueKind::String {
            choices: &["auto", "en-US", "zh-CN"],
        }),
        "ui.theme" => Ok(ValueKind::String {
            choices: &["auto", "light", "dark"],
        }),
        "ai.mode" => Ok(ValueKind::String {
            choices: &["native", "trusted-cli"],
        }),
        "ledger_dir" | "vault_path" => Ok(ValueKind::String { choices: &[] }),
        "ai.native_enabled"
        | "ai.agent_bridge.enabled"
        | "ai.agent_bridge.trusted"
        | "ui.sidebar_visible"
        | "ui.statusbar_visible"
        | "ui.outline_visible" => Ok(ValueKind::Bool),
        "snapshot_depth"
        | "concurrency"
        | "ai.agent_bridge.timeout_ms"
        | "ui.outline_width"
        | "ui.sidebar_width"
        | "ui.right_panel_width"
        | "ui.outer_gutter"
        | "ui.recent_commands_count"
        | "ui.recent_docs_count" => Ok(ValueKind::Integer),
        _ => Err(anyhow!("Unsupported config key: {}", key)),
    }
}

#[cfg(test)]
fn supported_config_keys() -> &'static [&'static str] {
    SUPPORTED_CONFIG_KEYS
}

enum ValueKind {
    String { choices: &'static [&'static str] },
    Bool,
    Integer,
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
        .with_context(|| format!("Invalid integer value: {}", value))?;
    if parsed < 0 {
        return Err(anyhow!("Integer config values must be non-negative"));
    }
    Ok(Value::Integer(parsed))
}

fn validate_config(output: &str) -> anyhow::Result<()> {
    toml::from_str::<deve_core::config::Config>(output)
        .context("Updated config.toml is not compatible with runtime config")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{set_in_file, supported_config_keys};
    use deve_core::config::{AppProfile, Config};
    use std::collections::BTreeSet;

    #[test]
    fn set_core_key_writes_runtime_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");

        set_in_file(&path, "profile", "low-spec").expect("set profile");

        let output = std::fs::read_to_string(path).expect("read config");
        let config: Config = toml::from_str(&output).expect("valid config");
        assert_eq!(config.profile, AppProfile::LowSpec);
    }

    #[test]
    fn set_ui_key_is_preserved_without_breaking_runtime_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");

        set_in_file(&path, "ui.sidebar_width", "300").expect("set ui");

        let output = std::fs::read_to_string(path).expect("read config");
        toml::from_str::<Config>(&output).expect("runtime-compatible config");
        assert!(output.contains("sidebar_width = 300"));
    }

    #[test]
    fn set_rejects_unknown_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let original = "profile = \"standard\"\n";
        std::fs::write(&path, original).expect("seed config");

        let err = set_in_file(&path, "unknown.key", "1").expect_err("reject key");
        assert!(err.to_string().contains("Unsupported config key"));

        let err =
            set_in_file(&path, "server.settings.api_enabled", "true").expect_err("reject future");
        assert!(err.to_string().contains("Unsupported config key"));
        assert_eq!(
            std::fs::read_to_string(&path).expect("read config"),
            original
        );
    }

    #[test]
    fn set_rejects_invalid_value_without_rewriting_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        let original = "profile = \"standard\"\n";
        std::fs::write(&path, original).expect("seed config");

        let invalid_choice = set_in_file(&path, "profile", "invalid").expect_err("reject choice");
        assert!(invalid_choice.to_string().contains("Invalid value"));
        assert_eq!(
            std::fs::read_to_string(&path).expect("read config"),
            original
        );

        let invalid_integer =
            set_in_file(&path, "ui.sidebar_width", "-1").expect_err("reject integer");
        assert!(
            invalid_integer
                .to_string()
                .contains("Integer config values must be non-negative")
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("read config"),
            original
        );
    }

    #[test]
    fn supported_config_keys_match_settings_plan_tables() {
        let docs = include_str!("../../../../docs/plan/13_settings.md");
        let documented = extract_documented_config_keys(docs);
        let supported = supported_config_keys()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();

        assert_eq!(documented, supported);
    }

    fn extract_documented_config_keys(docs: &str) -> BTreeSet<&str> {
        let mut keys = BTreeSet::new();
        let mut in_config_section = false;
        for line in docs.lines() {
            if line.starts_with("## 3.") {
                break;
            }
            if line.starts_with("### 2.") {
                in_config_section = true;
                continue;
            }
            if !in_config_section || !line.starts_with('|') {
                continue;
            }
            let Some(first_cell) = line.split('|').nth(1).map(str::trim) else {
                continue;
            };
            if !first_cell.starts_with('`') {
                continue;
            }
            for key in first_cell.split("<br>").flat_map(|cell| cell.split('/')) {
                let key = key.trim().trim_matches('`');
                if key.starts_with("DEVE_") || key.is_empty() {
                    continue;
                }
                keys.insert(key);
            }
        }
        keys
    }
}
