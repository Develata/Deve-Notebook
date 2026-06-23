//! plan_ref: infra

use crate::context::BaselineContext;
use crate::mobile_shell_gate::{assert_android_shell_boundary, binary_flag_from_env};
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "mobile-android-release-preflight-check";

pub fn run() -> Result<()> {
    binary_flag_from_env(
        LABEL,
        "DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED",
        false,
    )?;
    binary_flag_from_env(
        LABEL,
        "DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED",
        false,
    )?;
    let artifact_kind =
        env::var("DEVE_MOBILE_ANDROID_RELEASE_ARTIFACT_KIND").unwrap_or_else(|_| "aab".to_string());
    validate_artifact_kind(&artifact_kind)?;

    let ctx = BaselineContext::new(LABEL)?;
    assert_android_shell_boundary(
        ctx.root(),
        LABEL,
        "iOS generated project is not allowed in the Android release preflight",
    )?;
    ctx.ok();
    Ok(())
}

fn validate_artifact_kind(kind: &str) -> Result<()> {
    match kind {
        "apk" | "aab" => Ok(()),
        _ => bail!("{LABEL}: DEVE_MOBILE_ANDROID_RELEASE_ARTIFACT_KIND must be apk or aab"),
    }
}

#[cfg(test)]
mod tests {
    use super::{LABEL, validate_artifact_kind};
    use crate::mobile_shell_gate::parse_binary_flag;

    #[test]
    fn accepts_binary_android_release_preflight_flags() {
        for (name, value) in [
            ("DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED", "0"),
            ("DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED", "1"),
            (
                "DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED",
                "0",
            ),
            (
                "DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED",
                "1",
            ),
        ] {
            parse_binary_flag(LABEL, name, value).expect("binary flag");
        }
    }

    #[test]
    fn rejects_non_binary_android_release_preflight_flags() {
        for (name, value) in [
            ("DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED", "yes"),
            (
                "DEVE_MOBILE_ANDROID_PHYSICAL_DEVICE_PREFLIGHT_REQUIRED",
                "maybe",
            ),
            ("DEVE_MOBILE_ANDROID_RELEASE_PREFLIGHT_REQUIRED", "2"),
        ] {
            assert!(parse_binary_flag(LABEL, name, value).is_err());
        }
    }

    #[test]
    fn accepts_supported_android_release_artifact_kinds() {
        for kind in ["apk", "aab"] {
            validate_artifact_kind(kind).expect("supported artifact kind");
        }
    }

    #[test]
    fn rejects_unknown_android_release_artifact_kinds() {
        for kind in ["", "zip", "APK", " apk"] {
            assert!(validate_artifact_kind(kind).is_err());
        }
    }
}
