//! plan_ref: infra

use crate::context::BaselineContext;
use crate::mobile_shell_gate::assert_android_shell_boundary;
use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "mobile-android-release-preflight-check";

pub fn run() -> Result<()> {
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
    use super::validate_artifact_kind;

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
