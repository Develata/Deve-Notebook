//! plan_ref: infra

use crate::context::BaselineContext;
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "desktop-signing-preflight-check";

pub fn run() -> Result<()> {
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
    use super::validate_targets;

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
