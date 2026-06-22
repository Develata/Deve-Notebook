//! plan_ref: infra

use crate::context::BaselineContext;
use crate::mobile_shell_gate::assert_android_shell_boundary;
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "mobile-android-shell-package-build-check";

pub fn run() -> Result<()> {
    let target =
        env::var("DEVE_MOBILE_ANDROID_PACKAGE_TARGET").unwrap_or_else(|_| "aarch64".to_string());
    let build_apk = env::var("DEVE_MOBILE_ANDROID_PACKAGE_APK").unwrap_or_else(|_| "1".to_string());
    let build_aab = env::var("DEVE_MOBILE_ANDROID_PACKAGE_AAB").unwrap_or_else(|_| "0".to_string());
    let build_debug =
        env::var("DEVE_MOBILE_ANDROID_PACKAGE_DEBUG").unwrap_or_else(|_| "0".to_string());

    validate_target(&target)?;
    validate_artifact_kind(&build_apk, &build_aab, &build_debug)?;

    let ctx = BaselineContext::new(LABEL)?;
    assert_android_shell_boundary(
        ctx.root(),
        LABEL,
        "iOS generated project is not allowed in the Android shell package gate",
    )?;
    ctx.ok();
    Ok(())
}

fn validate_target(target: &str) -> Result<()> {
    match target {
        "aarch64" | "armv7" | "i686" | "x86_64" => Ok(()),
        _ => bail!("{LABEL}: unsupported Android target: {target}"),
    }
}

fn validate_artifact_kind(build_apk: &str, build_aab: &str, build_debug: &str) -> Result<()> {
    if !is_enabled(build_apk) && !is_enabled(build_aab) {
        bail!(
            "{LABEL}: at least one of DEVE_MOBILE_ANDROID_PACKAGE_APK or DEVE_MOBILE_ANDROID_PACKAGE_AAB must be 1"
        );
    }

    if is_enabled(build_debug) && is_enabled(build_aab) {
        bail!(
            "{LABEL}: debug Android install-smoke builds must produce APK only; AAB is release/store packaging"
        );
    }

    Ok(())
}

fn is_enabled(value: &str) -> bool {
    value == "1"
}

#[cfg(test)]
mod tests {
    use super::{validate_artifact_kind, validate_target};

    #[test]
    fn accepts_supported_android_package_targets() {
        for target in ["aarch64", "armv7", "i686", "x86_64"] {
            validate_target(target).expect("supported target");
        }
    }

    #[test]
    fn rejects_unknown_android_package_target() {
        assert!(validate_target("mips64").is_err());
    }

    #[test]
    fn rejects_android_package_without_artifact_kind() {
        assert!(validate_artifact_kind("0", "0", "0").is_err());
    }

    #[test]
    fn rejects_debug_android_package_with_aab() {
        assert!(validate_artifact_kind("1", "1", "1").is_err());
    }
}
