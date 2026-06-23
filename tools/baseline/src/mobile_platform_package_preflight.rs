//! plan_ref: infra

use crate::context::BaselineContext;
use crate::mobile_shell_gate::binary_flag_from_env;
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "mobile-platform-package-preflight-check";

pub fn run() -> Result<()> {
    binary_flag_from_env(LABEL, "DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED", false)?;
    binary_flag_from_env(
        LABEL,
        "DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK",
        true,
    )?;
    let targets =
        env::var("DEVE_MOBILE_PACKAGE_TARGETS").unwrap_or_else(|_| "android,ios".to_string());

    let android_enabled = validate_targets(&targets)?;
    if android_enabled {
        let android_target = env::var("DEVE_MOBILE_ANDROID_PACKAGE_TARGET")
            .unwrap_or_else(|_| "aarch64".to_string());
        validate_android_target(&android_target)?;
    }

    let ctx = BaselineContext::new(LABEL)?;
    ctx.ok();
    Ok(())
}

fn validate_targets(targets: &str) -> Result<bool> {
    let mut count = 0usize;
    let mut android_enabled = false;

    for target in targets.split(',').map(str::trim) {
        if target.is_empty() {
            bail!("{LABEL}: DEVE_MOBILE_PACKAGE_TARGETS contains an empty target");
        }

        match target {
            "android" => {
                count += 1;
                android_enabled = true;
            }
            "ios" => count += 1,
            _ => bail!("{LABEL}: unsupported mobile package target: {target}"),
        }
    }

    if count == 0 {
        bail!("{LABEL}: DEVE_MOBILE_PACKAGE_TARGETS must include android or ios");
    }

    Ok(android_enabled)
}

fn validate_android_target(target: &str) -> Result<()> {
    match target {
        "aarch64" | "armv7" | "i686" | "x86_64" => Ok(()),
        _ => bail!("{LABEL}: unsupported Android package target: {target}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{LABEL, validate_android_target, validate_targets};
    use crate::mobile_shell_gate::parse_binary_flag;

    #[test]
    fn accepts_binary_mobile_package_preflight_flags() {
        for (name, value) in [
            ("DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED", "0"),
            ("DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED", "1"),
            ("DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK", "0"),
            ("DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK", "1"),
        ] {
            parse_binary_flag(LABEL, name, value).expect("binary flag");
        }
    }

    #[test]
    fn rejects_non_binary_mobile_package_preflight_flags() {
        for (name, value) in [
            ("DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED", "yes"),
            ("DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK", "maybe"),
            ("DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED", "2"),
        ] {
            assert!(parse_binary_flag(LABEL, name, value).is_err());
        }
    }

    #[test]
    fn accepts_supported_mobile_package_targets() {
        for targets in ["android", "ios", "android,ios", " android , ios "] {
            validate_targets(targets).expect("supported target list");
        }
    }

    #[test]
    fn rejects_unsupported_mobile_package_targets() {
        for targets in ["", "linux", "android,linux", "android,", ",ios"] {
            assert!(validate_targets(targets).is_err());
        }
    }

    #[test]
    fn accepts_supported_android_package_targets() {
        for target in ["aarch64", "armv7", "i686", "x86_64"] {
            validate_android_target(target).expect("supported Android target");
        }
    }

    #[test]
    fn rejects_unsupported_android_package_targets() {
        assert!(validate_android_target("mips64").is_err());
    }
}
