//! Typed YAML accessors with path-rich fail-closed diagnostics.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use anyhow::{Context, Result, bail};
use yaml_rust2::Yaml;
use yaml_rust2::yaml::Hash;

pub(super) fn required<'a>(mapping: &'a Hash, key: &str, path: &str) -> Result<&'a Yaml> {
    optional(mapping, key).with_context(|| format!("acceptance producers: {path} is missing {key}"))
}

pub(super) fn optional<'a>(mapping: &'a Hash, key: &str) -> Option<&'a Yaml> {
    mapping.get(&Yaml::String(key.to_owned()))
}

pub(super) fn as_mapping<'a>(value: &'a Yaml, path: &str) -> Result<&'a Hash> {
    value
        .as_hash()
        .with_context(|| format!("acceptance producers: {path} must be a mapping"))
}

pub(super) fn as_sequence<'a>(value: &'a Yaml, path: &str) -> Result<&'a [Yaml]> {
    value
        .as_vec()
        .map(Vec::as_slice)
        .with_context(|| format!("acceptance producers: {path} must be a sequence"))
}

pub(super) fn as_string<'a>(value: &'a Yaml, path: &str) -> Result<&'a str> {
    value
        .as_str()
        .with_context(|| format!("acceptance producers: {path} must be a string"))
}

pub(super) fn scalar_text(value: &Yaml) -> Result<String> {
    match value {
        Yaml::String(value) => Ok(value.clone()),
        Yaml::Integer(value) => Ok(value.to_string()),
        _ => bail!("acceptance producers: setup action input must be a string or integer"),
    }
}

pub(super) fn as_u64(value: &Yaml, path: &str) -> Result<u64> {
    value
        .as_i64()
        .filter(|value| *value > 0)
        .map(|value| value as u64)
        .with_context(|| format!("acceptance producers: {path} must be a positive integer"))
}
