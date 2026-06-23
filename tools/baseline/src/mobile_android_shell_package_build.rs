//! plan_ref: infra

use crate::context::BaselineContext;
use crate::env_gate::binary_flag_from_env;
use crate::mobile_shell_gate::assert_android_shell_boundary;
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "mobile-android-shell-package-build-check";

pub fn run() -> Result<()> {
    let target =
        env::var("DEVE_MOBILE_ANDROID_PACKAGE_TARGET").unwrap_or_else(|_| "aarch64".to_string());
    let build_apk = binary_flag_from_env(LABEL, "DEVE_MOBILE_ANDROID_PACKAGE_APK", true)?;
    let build_aab = binary_flag_from_env(LABEL, "DEVE_MOBILE_ANDROID_PACKAGE_AAB", false)?;
    let build_debug = binary_flag_from_env(LABEL, "DEVE_MOBILE_ANDROID_PACKAGE_DEBUG", false)?;

    validate_target(&target)?;
    validate_artifact_kind(build_apk, build_aab, build_debug)?;

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

fn validate_artifact_kind(build_apk: bool, build_aab: bool, build_debug: bool) -> Result<()> {
    if !build_apk && !build_aab {
        bail!(
            "{LABEL}: at least one of DEVE_MOBILE_ANDROID_PACKAGE_APK or DEVE_MOBILE_ANDROID_PACKAGE_AAB must be 1"
        );
    }

    if build_debug && build_aab {
        bail!(
            "{LABEL}: debug Android install-smoke builds must produce APK only; AAB is release/store packaging"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LABEL, validate_artifact_kind, validate_target};
    use crate::env_gate::parse_binary_flag;

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
        assert!(validate_artifact_kind(false, false, false).is_err());
    }

    #[test]
    fn rejects_debug_android_package_with_aab() {
        assert!(validate_artifact_kind(true, true, true).is_err());
    }

    #[test]
    fn rejects_non_binary_android_package_flags() {
        for (name, value) in [
            ("DEVE_MOBILE_ANDROID_PACKAGE_APK", "2"),
            ("DEVE_MOBILE_ANDROID_PACKAGE_AAB", "true"),
            ("DEVE_MOBILE_ANDROID_PACKAGE_DEBUG", "yes"),
        ] {
            assert!(parse_binary_flag(LABEL, name, value).is_err());
        }
    }
}
