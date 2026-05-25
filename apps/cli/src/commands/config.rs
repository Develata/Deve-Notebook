//! plan_ref:
//!   - 14_commands#cli-commands
//!   - 15_settings#configuration-settings

use anyhow::{Context, anyhow};
use std::path::Path;
use toml::Value;
use toml::value::Table;

mod schema;

const CONFIG_FILE: &str = "config.toml";

pub fn print(config: &deve_core::config::Config) -> anyhow::Result<()> {
    let output = toml::to_string_pretty(config).context("Failed to render config")?;
    print!("{output}");
    Ok(())
}

pub fn set(key: &str, value: &str) -> anyhow::Result<()> {
    set_in_file(Path::new(CONFIG_FILE), key, value)
}

fn set_in_file(path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
    let parsed = schema::parse_whitelisted_value(key, value)?;
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

fn validate_config(output: &str) -> anyhow::Result<()> {
    toml::from_str::<deve_core::config::Config>(output)
        .context("Updated config.toml is not compatible with runtime config")?;
    Ok(())
}

#[cfg(test)]
mod tests;
