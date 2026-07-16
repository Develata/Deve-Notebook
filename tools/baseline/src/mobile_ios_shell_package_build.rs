//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-ios-shell-package-execution-gate

use crate::context::BaselineContext;
use crate::env_gate::binary_flag_from_env;
use crate::mobile_shell_gate::assert_ios_shell_boundary;
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "mobile-ios-shell-package-build-check";

pub fn run() -> Result<()> {
    binary_flag_from_env(LABEL, "DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED", false)?;
    let target =
        env::var("DEVE_MOBILE_IOS_PACKAGE_TARGET").unwrap_or_else(|_| "aarch64-sim".to_string());
    validate_target(&target)?;

    let ctx = BaselineContext::new(LABEL)?;
    assert_ios_shell_boundary(ctx.root(), LABEL)?;
    ctx.ok();
    Ok(())
}

fn validate_target(target: &str) -> Result<()> {
    match target {
        "aarch64" | "aarch64-sim" | "x86_64" => Ok(()),
        _ => bail!("{LABEL}: unsupported iOS target: {target}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{LABEL, validate_target};
    use crate::env_gate::parse_binary_flag;

    #[test]
    fn accepts_binary_ios_package_required_flag() {
        for required in ["0", "1"] {
            parse_binary_flag(LABEL, "DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED", required)
                .expect("binary required flag");
        }
    }

    #[test]
    fn rejects_non_binary_ios_package_required_flag() {
        for required in ["", "true", "yes", "2"] {
            assert!(
                parse_binary_flag(LABEL, "DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED", required)
                    .is_err()
            );
        }
    }

    #[test]
    fn accepts_supported_ios_package_targets() {
        for target in ["aarch64", "aarch64-sim", "x86_64"] {
            validate_target(target).expect("supported target");
        }
    }

    #[test]
    fn rejects_unknown_ios_package_target() {
        assert!(validate_target("arm64-simulator").is_err());
    }
}
