//! plan_ref: infra

use anyhow::{Context, Result, bail};
use std::env;
use std::path::Path;

pub fn assert_android_shell_boundary(
    root: &Path,
    label: &str,
    apple_generated_message: &str,
) -> Result<()> {
    forbid_path(
        root,
        label,
        "apps/mobile/gen/apple",
        apple_generated_message,
    )?;
    assert_common_mobile_shell_boundary(root, label)
}

pub fn assert_ios_shell_boundary(root: &Path, label: &str) -> Result<()> {
    assert_common_mobile_shell_boundary(root, label)
}

pub fn assert_positive_integer(label: &str, name: &str, value: &str) -> Result<()> {
    if is_positive_integer(value) {
        return Ok(());
    }

    bail!("{label}: {name} must be a positive integer");
}

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

fn assert_common_mobile_shell_boundary(root: &Path, label: &str) -> Result<()> {
    forbid_path(
        root,
        label,
        "apps/mobile/src-tauri",
        "legacy src-tauri layout is not allowed for apps/mobile",
    )?;
    forbid_path(
        root,
        label,
        "apps/mobile/src/main.rs",
        "mobile shell must expose the Tauri mobile entrypoint from lib.rs, not src/main.rs",
    )
}

fn forbid_path(root: &Path, label: &str, rel: &str, message: &str) -> Result<()> {
    if root
        .join(rel)
        .try_exists()
        .with_context(|| format!("{label}: failed to inspect {rel}"))?
    {
        bail!("{label}: {message}");
    }
    Ok(())
}

fn is_positive_integer(value: &str) -> bool {
    let bytes = value.as_bytes();
    matches!(bytes.first(), Some(b'1'..=b'9')) && bytes.iter().all(u8::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::{assert_positive_integer, parse_binary_flag};

    #[test]
    fn accepts_positive_integer_values() {
        for value in ["1", "3", "60", "120"] {
            assert_positive_integer("test-check", "TEST_VALUE", value).expect("positive integer");
        }
    }

    #[test]
    fn rejects_non_positive_integer_values() {
        for value in ["", "0", "00", "-1", "1.5", "abc"] {
            assert!(assert_positive_integer("test-check", "TEST_VALUE", value).is_err());
        }
    }

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
