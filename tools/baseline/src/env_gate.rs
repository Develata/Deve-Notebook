//! plan_ref: infra

use anyhow::{Result, bail};
use std::env;

pub fn parse_binary_flag(label: &str, name: &str, value: &str) -> Result<bool> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => bail!("{label}: {name} must be 0 or 1"),
    }
}

pub fn binary_flag_from_env(label: &str, name: &str, default: bool) -> Result<bool> {
    match env::var(name) {
        Ok(value) => parse_binary_flag(label, name, &value),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => bail!("{label}: {name} must be valid Unicode"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_binary_flag;

    #[test]
    fn parses_binary_flags() {
        assert!(!parse_binary_flag("test-check", "TEST_FLAG", "0").expect("flag"));
        assert!(parse_binary_flag("test-check", "TEST_FLAG", "1").expect("flag"));
    }

    #[test]
    fn rejects_non_binary_flags() {
        for value in ["", "true", "false", "yes", "2"] {
            assert!(parse_binary_flag("test-check", "TEST_FLAG", value).is_err());
        }
    }
}
