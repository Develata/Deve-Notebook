//! plan_ref: infra

use crate::context::BaselineContext;
use crate::mobile_shell_gate::assert_ios_shell_boundary;
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "mobile-ios-shell-package-build-check";

pub fn run() -> Result<()> {
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
    use super::validate_target;

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
