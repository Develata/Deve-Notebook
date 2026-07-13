//! plan_ref: infra

use crate::context::BaselineContext;
use crate::env_gate::binary_flag_from_env;
use anyhow::{Result, bail};
use std::env;

mod android;

pub use self::android::run_mobile_android_emulator_install_startup_smoke;

const QUICK_LABEL: &str = "local-quick-gate";
const DEEP_LABEL: &str = "deep-audit-gate";
const DESKTOP_PLATFORM_LABEL: &str = "desktop-platform-package-build-check";
const DESKTOP_STARTUP_LABEL: &str = "desktop-package-startup-smoke-check";
const DESKTOP_NATIVE_SESSION_LABEL: &str = "desktop-native-session-package-smoke-check";
const DESKTOP_INSTALLER_LABEL: &str = "desktop-installer-smoke-check";

pub fn run_local_quick_gate() -> Result<()> {
    flag_from_env(QUICK_LABEL, "DEVE_QUICK_GATE_TESTS", true)?;
    local_quick_gate_target_dir_from_env()?;
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
    desktop_bundles_from_env(DESKTOP_STARTUP_LABEL, BundlePolicy::PackageSmoke)?;
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
    desktop_bundles_from_env(DESKTOP_NATIVE_SESSION_LABEL, BundlePolicy::PackageSmoke)?;
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

fn ok(label: &'static str) -> Result<()> {
    let ctx = BaselineContext::new(label)?;
    ctx.ok();
    Ok(())
}

fn flag_from_env(label: &str, name: &str, default: bool) -> Result<bool> {
    binary_flag_from_env(label, name, default)
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

fn local_quick_gate_target_dir_from_env() -> Result<String> {
    match env::var("DEVE_LOCAL_QUICK_GATE_TARGET_DIR") {
        Ok(value) => validate_local_quick_gate_target_dir(&value),
        Err(env::VarError::NotPresent) => {
            validate_local_quick_gate_target_dir("target/local-quick-gate")
        }
        Err(env::VarError::NotUnicode(_)) => {
            bail!("{QUICK_LABEL}: DEVE_LOCAL_QUICK_GATE_TARGET_DIR must be valid Unicode")
        }
    }
}

fn validate_local_quick_gate_target_dir(value: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("{QUICK_LABEL}: DEVE_LOCAL_QUICK_GATE_TARGET_DIR must not be empty");
    }
    if value != value.trim() {
        bail!("{QUICK_LABEL}: DEVE_LOCAL_QUICK_GATE_TARGET_DIR must not contain outer whitespace");
    }
    if value.chars().any(|ch| ch.is_ascii_control()) {
        bail!(
            "{QUICK_LABEL}: DEVE_LOCAL_QUICK_GATE_TARGET_DIR must not contain control characters"
        );
    }

    let normalized = value.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    let relative = trimmed
        .strip_prefix("./")
        .unwrap_or(trimmed)
        .to_ascii_lowercase();
    if relative == "target" || relative == "target/debug" {
        bail!(
            "{QUICK_LABEL}: DEVE_LOCAL_QUICK_GATE_TARGET_DIR must not point at the shared default cargo target directory"
        );
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
    PackageSmoke,
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
            | (
                BundlePolicy::PackageSmoke,
                "app" | "dmg" | "msi" | "nsis" | "deb" | "rpm" | "appimage" | "exe",
            )
            | (BundlePolicy::InstallerSmoke, "app" | "dmg" | "msi" | "nsis") => {
                parsed.push(bundle);
            }
            (_, "") => match policy {
                BundlePolicy::PackageBuild | BundlePolicy::PackageSmoke => {
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
                BundlePolicy::PackageBuild | BundlePolicy::PackageSmoke => {
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

#[cfg(test)]
mod tests {
    use super::{
        BundlePolicy, DESKTOP_INSTALLER_LABEL, DESKTOP_PLATFORM_LABEL, DESKTOP_STARTUP_LABEL,
        parse_positive_integer, validate_desktop_bundles, validate_local_quick_gate_target_dir,
    };
    use crate::env_gate::parse_binary_flag;

    #[test]
    fn gate_flags_accept_only_binary_values() {
        assert!(!parse_binary_flag("gate", "DEVE_FLAG", "0").expect("flag"));
        assert!(parse_binary_flag("gate", "DEVE_FLAG", "1").expect("flag"));

        for value in ["", "true", "false", "yes", "2"] {
            assert!(parse_binary_flag("gate", "DEVE_FLAG", value).is_err());
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
    fn local_quick_gate_target_dir_stays_isolated_from_default_target() {
        for value in [
            "target/local-quick-gate",
            "target\\local-quick-gate",
            "target/local-quick-gate-custom",
            "/tmp/deve-local-quick-gate",
            "E:\\deve\\local-quick-gate",
        ] {
            validate_local_quick_gate_target_dir(value).expect("isolated target dir");
        }

        for value in [
            "",
            " target/local-quick-gate",
            "target ",
            "target",
            "target/",
            "./target",
            "target/debug",
            "target\\debug",
            "target/local\nquick",
        ] {
            validate_local_quick_gate_target_dir(value).expect_err("shared or invalid target dir");
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

        assert!(
            validate_desktop_bundles(DESKTOP_PLATFORM_LABEL, BundlePolicy::PackageBuild, "exe")
                .is_err()
        );
    }

    #[test]
    fn package_smoke_bundle_policy_accepts_release_binary_probe() {
        validate_desktop_bundles(
            DESKTOP_STARTUP_LABEL,
            BundlePolicy::PackageSmoke,
            " exe, msi,nsis ",
        )
        .expect("startup smoke supports release binary and package probes");
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
        assert!(
            validate_desktop_bundles(DESKTOP_INSTALLER_LABEL, BundlePolicy::InstallerSmoke, "exe")
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
}
