//! plan_ref: infra

use crate::context::BaselineContext;
use crate::env_gate::binary_flag_from_env;
use anyhow::{Context, Result, bail};
use std::env;

const LABEL: &str = "desktop-target-host-preflight-check";
const REQUIRED_FILES: &[&str] = &[
    "apps/desktop/tauri.conf.json",
    "apps/desktop/src/main.rs",
    "apps/desktop/build.rs",
    "apps/desktop/icons/icon.png",
];

pub fn run() -> Result<()> {
    binary_flag_from_env(LABEL, "DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED", false)?;
    binary_flag_from_env(LABEL, "DEVE_DESKTOP_PACKAGE_NO_SIGN", false)?;
    let targets =
        env::var("DEVE_DESKTOP_TARGET_HOSTS").unwrap_or_else(|_| "macos,windows".to_string());
    validate_targets(&targets)?;

    let ctx = BaselineContext::new(LABEL)?;
    validate_required_files(&ctx)?;
    ctx.ok();
    Ok(())
}

fn validate_targets(targets: &str) -> Result<()> {
    let mut has_target = false;

    for raw_target in targets.split(',') {
        let target = normalize_target(raw_target);
        match target.as_str() {
            "macos" | "windows" => has_target = true,
            "" => bail!("{LABEL}: DEVE_DESKTOP_TARGET_HOSTS must list macos or windows"),
            _ => bail!(
                "{LABEL}: DEVE_DESKTOP_TARGET_HOSTS must list only macos or windows; invalid target: {target}"
            ),
        }
    }

    if has_target {
        Ok(())
    } else {
        bail!("{LABEL}: DEVE_DESKTOP_TARGET_HOSTS must list macos or windows")
    }
}

fn normalize_target(target: &str) -> String {
    target.trim().to_string()
}

fn validate_required_files(ctx: &BaselineContext) -> Result<()> {
    for rel in REQUIRED_FILES {
        let path = ctx.root().join(rel);
        if !path
            .try_exists()
            .with_context(|| format!("{LABEL}: failed to check {rel}"))?
            || !path.is_file()
        {
            bail!("{LABEL}: invalid {rel}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LABEL, validate_targets};
    use crate::env_gate::parse_binary_flag;

    #[test]
    fn accepts_binary_desktop_target_host_flags() {
        for (name, value) in [
            ("DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED", "0"),
            ("DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED", "1"),
            ("DEVE_DESKTOP_PACKAGE_NO_SIGN", "0"),
            ("DEVE_DESKTOP_PACKAGE_NO_SIGN", "1"),
        ] {
            parse_binary_flag(LABEL, name, value).expect("binary flag");
        }
    }

    #[test]
    fn rejects_non_binary_desktop_target_host_flags() {
        for (name, value) in [
            ("DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED", "yes"),
            ("DEVE_DESKTOP_PACKAGE_NO_SIGN", "maybe"),
            ("DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED", "2"),
        ] {
            assert!(parse_binary_flag(LABEL, name, value).is_err());
        }
    }

    #[test]
    fn accepts_supported_desktop_target_hosts() {
        for targets in ["macos", "windows", "macos,windows", " macos , windows "] {
            validate_targets(targets).expect("supported target list");
        }
    }

    #[test]
    fn rejects_unsupported_desktop_target_hosts() {
        for targets in ["", "linux", "macos,linux", "macos,", ",windows"] {
            assert!(validate_targets(targets).is_err());
        }
    }

    #[test]
    fn rejects_internal_whitespace_in_desktop_target_hosts() {
        for targets in ["ma cos", "win dows", "macos, win dows"] {
            assert!(validate_targets(targets).is_err());
        }
    }
}
