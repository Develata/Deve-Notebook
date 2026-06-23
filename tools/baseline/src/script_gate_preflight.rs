//! plan_ref: infra

use crate::context::BaselineContext;
use anyhow::{Result, bail};
use std::env;

const QUICK_LABEL: &str = "local-quick-gate";
const DEEP_LABEL: &str = "deep-audit-gate";
const DESKTOP_PLATFORM_LABEL: &str = "desktop-platform-package-build-check";
const DESKTOP_STARTUP_LABEL: &str = "desktop-package-startup-smoke-check";
const DESKTOP_NATIVE_SESSION_LABEL: &str = "desktop-native-session-package-smoke-check";
const DESKTOP_INSTALLER_LABEL: &str = "desktop-installer-smoke-check";
const ANDROID_EMULATOR_LABEL: &str = "mobile-android-emulator-install-startup-smoke-check";

pub fn run_local_quick_gate() -> Result<()> {
    flag_from_env(QUICK_LABEL, "DEVE_QUICK_GATE_TESTS", true)?;
    ok(QUICK_LABEL)
}

pub fn run_deep_audit_gate() -> Result<()> {
    flag_from_env(DEEP_LABEL, "DEVE_DEEP_AUDIT_WRITE_REPORT", false)?;
    flag_from_env(DEEP_LABEL, "DEVE_DEEP_AUDIT_FULL_TESTS", false)?;
    flag_from_env(DEEP_LABEL, "DEVE_DEEP_AUDIT_DOCKER_SMOKE", false)?;
    ok(DEEP_LABEL)
}

pub fn run_desktop_platform_package_build() -> Result<()> {
    flag_from_env(
        DESKTOP_PLATFORM_LABEL,
        "DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED",
        false,
    )?;
    flag_from_env(
        DESKTOP_PLATFORM_LABEL,
        "DEVE_DESKTOP_PACKAGE_NO_SIGN",
        false,
    )?;
    desktop_bundles_from_env(DESKTOP_PLATFORM_LABEL, BundlePolicy::PackageBuild)?;
    non_empty_string_from_env(
        DESKTOP_PLATFORM_LABEL,
        "DEVE_DESKTOP_PACKAGE_FEATURES",
        "native-packaging",
    )?;
    ok(DESKTOP_PLATFORM_LABEL)
}

pub fn run_desktop_package_startup_smoke() -> Result<()> {
    flag_from_env(
        DESKTOP_STARTUP_LABEL,
        "DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED",
        false,
    )?;
    desktop_bundles_from_env(DESKTOP_STARTUP_LABEL, BundlePolicy::PackageBuild)?;
    positive_integer_from_env(
        DESKTOP_STARTUP_LABEL,
        "DEVE_DESKTOP_STARTUP_SMOKE_TIMEOUT_SECS",
        "20",
    )?;
    ok(DESKTOP_STARTUP_LABEL)
}

pub fn run_desktop_native_session_package_smoke() -> Result<()> {
    flag_from_env(
        DESKTOP_NATIVE_SESSION_LABEL,
        "DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED",
        false,
    )?;
    desktop_bundles_from_env(DESKTOP_NATIVE_SESSION_LABEL, BundlePolicy::PackageBuild)?;
    native_session_timeout_from_env()?;
    ok(DESKTOP_NATIVE_SESSION_LABEL)
}

pub fn run_desktop_installer_smoke() -> Result<()> {
    flag_from_env(
        DESKTOP_INSTALLER_LABEL,
        "DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED",
        false,
    )?;
    desktop_bundles_from_env(DESKTOP_INSTALLER_LABEL, BundlePolicy::InstallerSmoke)?;
    positive_integer_from_env(
        DESKTOP_INSTALLER_LABEL,
        "DEVE_DESKTOP_STARTUP_SMOKE_TIMEOUT_SECS",
        "20",
    )?;
    positive_integer_from_env(
        DESKTOP_INSTALLER_LABEL,
        "DEVE_DESKTOP_INSTALLER_SMOKE_TIMEOUT_SECS",
        "180",
    )?;
    positive_integer_from_env(
        DESKTOP_INSTALLER_LABEL,
        "DEVE_DESKTOP_INSTALLER_SMOKE_KILL_AFTER_SECS",
        "10",
    )?;
    non_empty_string_from_env(
        DESKTOP_INSTALLER_LABEL,
        "DEVE_DESKTOP_INSTALLER_SMOKE_WORK_DIR",
        "target/desktop-installer-smoke",
    )?;
    ok(DESKTOP_INSTALLER_LABEL)
}

pub fn run_mobile_android_emulator_install_startup_smoke() -> Result<()> {
    flag_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED",
        false,
    )?;
    positive_integer_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL",
        "35",
    )?;
    non_empty_string_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_SYSTEM_TARGET",
        "default",
    )?;
    non_empty_string_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_ARCH",
        "x86_64",
    )?;
    non_empty_string_from_env(
        ANDROID_EMULATOR_LABEL,
        "DEVE_MOBILE_ANDROID_EMULATOR_AVD_NAME",
        "deve-mobile-smoke-api35-default-x86_64",
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
    android_package_target_from_env(ANDROID_EMULATOR_LABEL)?;
    ok(ANDROID_EMULATOR_LABEL)
}

fn ok(label: &'static str) -> Result<()> {
    let ctx = BaselineContext::new(label)?;
    ctx.ok();
    Ok(())
}

fn flag_from_env(label: &str, name: &str, default: bool) -> Result<bool> {
    match env::var(name) {
        Ok(value) => parse_flag(label, name, &value),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => bail!("{label}: {name} must be valid Unicode"),
    }
}

fn parse_flag(label: &str, name: &str, value: &str) -> Result<bool> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => bail!("{label}: {name} must be 0 or 1"),
    }
}

fn positive_integer_from_env(label: &str, name: &str, default: &str) -> Result<u64> {
    match env::var(name) {
        Ok(value) => parse_positive_integer(label, name, &value),
        Err(env::VarError::NotPresent) => parse_positive_integer(label, name, default),
        Err(env::VarError::NotUnicode(_)) => bail!("{label}: {name} must be valid Unicode"),
    }
}

fn parse_positive_integer(label: &str, name: &str, value: &str) -> Result<u64> {
    if matches!(value.as_bytes(), [b'1'..=b'9', rest @ ..] if rest.iter().all(u8::is_ascii_digit)) {
        let parsed: u64 = value
            .parse()
            .map_err(|_| anyhow::anyhow!("{label}: {name} must be a positive integer"))?;
        return Ok(parsed);
    }
    bail!("{label}: {name} must be a positive integer")
}

fn non_empty_string_from_env(label: &str, name: &str, default: &str) -> Result<String> {
    match env::var(name) {
        Ok(value) => validate_non_empty(label, name, &value),
        Err(env::VarError::NotPresent) => validate_non_empty(label, name, default),
        Err(env::VarError::NotUnicode(_)) => bail!("{label}: {name} must be valid Unicode"),
    }
}

fn validate_non_empty(label: &str, name: &str, value: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("{label}: {name} must not be empty");
    }
    Ok(value.to_string())
}

fn native_session_timeout_from_env() -> Result<u64> {
    match env::var("DEVE_DESKTOP_NATIVE_SESSION_SMOKE_TIMEOUT_SECS") {
        Ok(value) => parse_positive_integer(
            DESKTOP_NATIVE_SESSION_LABEL,
            "DEVE_DESKTOP_NATIVE_SESSION_SMOKE_TIMEOUT_SECS",
            &value,
        ),
        Err(env::VarError::NotPresent) => positive_integer_from_env(
            DESKTOP_NATIVE_SESSION_LABEL,
            "DEVE_DESKTOP_STARTUP_SMOKE_TIMEOUT_SECS",
            "30",
        ),
        Err(env::VarError::NotUnicode(_)) => bail!(
            "{DESKTOP_NATIVE_SESSION_LABEL}: DEVE_DESKTOP_NATIVE_SESSION_SMOKE_TIMEOUT_SECS must be valid Unicode"
        ),
    }
}

#[derive(Clone, Copy)]
enum BundlePolicy {
    PackageBuild,
    InstallerSmoke,
}

fn desktop_bundles_from_env(label: &str, policy: BundlePolicy) -> Result<Vec<String>> {
    match env::var("DEVE_DESKTOP_PACKAGE_BUNDLES") {
        Ok(value) => validate_desktop_bundles(label, policy, &value),
        Err(env::VarError::NotPresent) => Ok(Vec::new()),
        Err(env::VarError::NotUnicode(_)) => {
            bail!("{label}: DEVE_DESKTOP_PACKAGE_BUNDLES must be valid Unicode")
        }
    }
}

fn validate_desktop_bundles(
    label: &str,
    policy: BundlePolicy,
    bundles: &str,
) -> Result<Vec<String>> {
    if bundles.is_empty() {
        return Ok(Vec::new());
    }

    let mut parsed = Vec::new();
    for raw_bundle in bundles.split(',') {
        let bundle = normalize_selector(raw_bundle);
        match (policy, bundle.as_str()) {
            (
                BundlePolicy::PackageBuild,
                "app" | "dmg" | "msi" | "nsis" | "deb" | "rpm" | "appimage",
            )
            | (BundlePolicy::InstallerSmoke, "app" | "dmg" | "msi" | "nsis") => {
                parsed.push(bundle);
            }
            (_, "") => match policy {
                BundlePolicy::PackageBuild => {
                    bail!(
                        "{label}: empty desktop package bundle selector in DEVE_DESKTOP_PACKAGE_BUNDLES"
                    )
                }
                BundlePolicy::InstallerSmoke => {
                    bail!(
                        "{label}: empty desktop installer bundle selector in DEVE_DESKTOP_PACKAGE_BUNDLES"
                    )
                }
            },
            _ => match policy {
                BundlePolicy::PackageBuild => {
                    bail!("{label}: unsupported desktop package bundle selector: {bundle}")
                }
                BundlePolicy::InstallerSmoke => {
                    bail!("{label}: unsupported desktop installer bundle selector: {bundle}")
                }
            },
        }
    }
    Ok(parsed)
}

fn normalize_selector(value: &str) -> String {
    value.trim().to_string()
}

fn android_package_target_from_env(label: &str) -> Result<String> {
    match env::var("DEVE_MOBILE_ANDROID_PACKAGE_TARGET") {
        Ok(value) => validate_android_package_target(label, &value),
        Err(env::VarError::NotPresent) => validate_android_package_target(label, "x86_64"),
        Err(env::VarError::NotUnicode(_)) => {
            bail!("{label}: DEVE_MOBILE_ANDROID_PACKAGE_TARGET must be valid Unicode")
        }
    }
}

fn validate_android_package_target(label: &str, target: &str) -> Result<String> {
    match target {
        "aarch64" | "armv7" | "i686" | "x86_64" => Ok(target.to_string()),
        _ => bail!("{label}: unsupported Android target: {target}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ANDROID_EMULATOR_LABEL, BundlePolicy, DESKTOP_INSTALLER_LABEL, DESKTOP_PLATFORM_LABEL,
        parse_flag, parse_positive_integer, validate_android_package_target,
        validate_desktop_bundles,
    };

    #[test]
    fn gate_flags_accept_only_binary_values() {
        assert!(!parse_flag("gate", "DEVE_FLAG", "0").expect("flag"));
        assert!(parse_flag("gate", "DEVE_FLAG", "1").expect("flag"));

        for value in ["", "true", "false", "yes", "2"] {
            assert!(parse_flag("gate", "DEVE_FLAG", value).is_err());
        }
    }

    #[test]
    fn positive_integer_rejects_zero_empty_and_words() {
        assert_eq!(
            parse_positive_integer("gate", "DEVE_TIMEOUT", "120").expect("timeout"),
            120
        );

        for value in ["", "0", "01", "08", "-1", "1.5", "ten"] {
            assert!(parse_positive_integer("gate", "DEVE_TIMEOUT", value).is_err());
        }
    }

    #[test]
    fn package_build_bundle_policy_accepts_platform_bundle_set() {
        validate_desktop_bundles(
            DESKTOP_PLATFORM_LABEL,
            BundlePolicy::PackageBuild,
            " app, dmg ,msi,nsis,deb,rpm,appimage ",
        )
        .expect("supported bundles");
    }

    #[test]
    fn installer_bundle_policy_keeps_installer_subset() {
        validate_desktop_bundles(
            DESKTOP_INSTALLER_LABEL,
            BundlePolicy::InstallerSmoke,
            " app, dmg,msi,nsis ",
        )
        .expect("supported installer bundles");

        assert!(
            validate_desktop_bundles(DESKTOP_INSTALLER_LABEL, BundlePolicy::InstallerSmoke, "deb")
                .is_err()
        );
    }

    #[test]
    fn bundle_policy_rejects_empty_or_unknown_selectors() {
        for bundles in ["app,", ",msi", "bogus"] {
            assert!(
                validate_desktop_bundles(
                    DESKTOP_PLATFORM_LABEL,
                    BundlePolicy::PackageBuild,
                    bundles
                )
                .is_err()
            );
        }
    }

    #[test]
    fn bundle_policy_rejects_internal_whitespace_selectors() {
        for bundles in ["app image", "n s i s"] {
            assert!(
                validate_desktop_bundles(
                    DESKTOP_PLATFORM_LABEL,
                    BundlePolicy::PackageBuild,
                    bundles
                )
                .is_err()
            );
        }
    }

    #[test]
    fn android_package_target_matches_android_shell_gate() {
        for target in ["aarch64", "armv7", "i686", "x86_64"] {
            validate_android_package_target(ANDROID_EMULATOR_LABEL, target)
                .expect("supported Android target");
        }

        assert!(validate_android_package_target(ANDROID_EMULATOR_LABEL, "mips64").is_err());
    }
}
