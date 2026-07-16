//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity

use crate::context::BaselineContext;
use crate::env_gate::binary_flag_from_env;
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "desktop-signing-preflight-check";

pub fn run() -> Result<()> {
    binary_flag_from_env(LABEL, "DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED", false)?;
    let targets =
        env::var("DEVE_DESKTOP_SIGNING_TARGETS").unwrap_or_else(|_| "macos,windows".to_string());
    validate_targets(&targets)?;

    let ctx = BaselineContext::new(LABEL)?;
    ctx.ok();
    Ok(())
}

fn validate_targets(targets: &str) -> Result<()> {
    let mut count = 0usize;

    for target in targets.split(',').map(str::trim) {
        if target.is_empty() {
            bail!("{LABEL}: DEVE_DESKTOP_SIGNING_TARGETS contains an empty target");
        }

        match target {
            "macos" | "windows" => count += 1,
            _ => bail!("{LABEL}: unsupported desktop signing target: {target}"),
        }
    }

    if count == 0 {
        bail!("{LABEL}: DEVE_DESKTOP_SIGNING_TARGETS must include macos or windows");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LABEL, validate_targets};
    use crate::env_gate::parse_binary_flag;

    #[test]
    fn accepts_binary_desktop_signing_preflight_flag() {
        for value in ["0", "1"] {
            parse_binary_flag(LABEL, "DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED", value)
                .expect("binary flag");
        }
    }

    #[test]
    fn rejects_non_binary_desktop_signing_preflight_flag() {
        for value in ["", "true", "yes", "2"] {
            assert!(
                parse_binary_flag(LABEL, "DEVE_DESKTOP_SIGNING_PREFLIGHT_REQUIRED", value).is_err()
            );
        }
    }

    #[test]
    fn accepts_supported_desktop_signing_targets() {
        for targets in ["macos", "windows", "macos,windows", " macos , windows "] {
            validate_targets(targets).expect("supported target list");
        }
    }

    #[test]
    fn rejects_unsupported_desktop_signing_targets() {
        for targets in ["", "linux", "macos,linux", "macos,", ",windows"] {
            assert!(validate_targets(targets).is_err());
        }
    }
}
