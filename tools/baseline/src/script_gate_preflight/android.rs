//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-android-shell-package-execution-gate
//!
//! Android emulator/package preflight parsing.

use super::{flag_from_env, non_empty_string_from_env, ok, positive_integer_from_env};
use anyhow::{Result, bail};
use std::env;

const ANDROID_EMULATOR_LABEL: &str = "mobile-android-emulator-install-startup-smoke-check";

pub fn run_mobile_android_emulator_install_startup_smoke() -> Result<()> {
    flag_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED",
        false,
    )?;
    android_api_level_from_env()?;
    non_empty_string_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_SYSTEM_TARGET",
        "google_apis",
    )?;
    non_empty_string_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_ARCH",
        "x86_64",
    )?;
    non_empty_string_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_AVD_NAME",
        "deve-mobile-smoke-api36.1-google_apis-x86_64",
    )?;
    non_empty_string_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_DEVICE",
        "pixel_2",
    )?;
    positive_integer_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_BOOT_TIMEOUT_SECS",
        "900",
    )?;
    positive_integer_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS",
        "120",
    )?;
    positive_integer_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_LIFECYCLE_TIMEOUT_SECS",
        "600",
    )?;
    android_package_target_from_env()?;
    ok(ANDROID_EMULATOR_LABEL)
}

fn android_api_level_from_env() -> Result<String> {
    match env::var("DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL") {
        Ok(value) => validate_android_api_level(&value),
        Err(env::VarError::NotPresent) => validate_android_api_level("36.1"),
        Err(env::VarError::NotUnicode(_)) => bail!(
            "{ANDROID_EMULATOR_LABEL}: DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL must be valid Unicode"
        ),
    }
}

fn validate_android_api_level(value: &str) -> Result<String> {
    let mut components = value.split('.');
    let major = components.next().unwrap_or_default();
    let minor = components.next();
    let major_is_valid =
        matches!(major.as_bytes(), [b'1'..=b'9', rest @ ..] if rest.iter().all(u8::is_ascii_digit));
    let minor_is_valid =
        minor.is_none_or(|part| !part.is_empty() && part.as_bytes().iter().all(u8::is_ascii_digit));

    if major_is_valid && minor_is_valid && components.next().is_none() {
        return Ok(value.to_string());
    }
    bail!(
        "{ANDROID_EMULATOR_LABEL}: DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL must be a positive major or major.minor API level"
    )
}

fn android_package_target_from_env() -> Result<String> {
    match env::var("DEVE_MOBILE_ANDROID_PACKAGE_TARGET") {
        Ok(value) => validate_android_package_target(&value),
        Err(env::VarError::NotPresent) => validate_android_package_target("x86_64"),
        Err(env::VarError::NotUnicode(_)) => bail!(
            "{ANDROID_EMULATOR_LABEL}: DEVE_MOBILE_ANDROID_PACKAGE_TARGET must be valid Unicode"
        ),
    }
}

fn validate_android_package_target(target: &str) -> Result<String> {
    match target {
        "aarch64" | "armv7" | "i686" | "x86_64" => Ok(target.to_string()),
        _ => bail!("{ANDROID_EMULATOR_LABEL}: unsupported Android target: {target}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_android_api_level, validate_android_package_target};

    #[test]
    fn api_level_accepts_platform_major_and_minor_versions() {
        for value in ["35", "37.1"] {
            assert_eq!(validate_android_api_level(value).expect("API level"), value);
        }

        for value in ["", "0", "01", ".1", "37.", "37.1.1", "android-37.1"] {
            assert!(validate_android_api_level(value).is_err());
        }
    }

    #[test]
    fn package_target_matches_android_shell_gate() {
        for target in ["aarch64", "armv7", "i686", "x86_64"] {
            validate_android_package_target(target).expect("supported Android target");
        }

        assert!(validate_android_package_target("mips64").is_err());
    }
}
