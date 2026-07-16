//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

mod aggregate;
mod candidate;
mod native_candidate;
mod promotion;
mod text;

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub(super) const ANDROID_SIGNING_SECRETS: [&str; 4] = [
    "ANDROID_KEYSTORE_BASE64",
    "ANDROID_KEYSTORE_PASSWORD",
    "ANDROID_KEY_ALIAS",
    "ANDROID_KEY_PASSWORD",
];

pub(super) fn check(root: &Path) -> Result<()> {
    promotion::require_single_tag_entry(root)?;
    candidate::check(root)?;
    native_candidate::check(root)?;
    aggregate::check(root)?;
    promotion::check(root)?;
    Ok(())
}

pub(super) fn read_workflow(root: &Path, name: &str) -> Result<String> {
    let path = root.join(".github/workflows").join(name);
    fs::read_to_string(&path).with_context(|| format!("read workflow {}", path.display()))
}

#[cfg(test)]
use native_candidate::{validate_webview2_cdp_arguments, validated_mobile_android_job};

#[cfg(test)]
mod tests {
    use super::{validate_webview2_cdp_arguments, validated_mobile_android_job};

    #[test]
    fn android_candidate_disables_unrelated_linux_host_packaging_check() {
        let valid = r#"jobs:
  mobile-android:
    env:
      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: "0"
    steps: []
"#;
        validated_mobile_android_job(valid).expect("valid Android job");

        for invalid in [
            "jobs:\n  mobile-android:\n    steps: []\n",
            "jobs:\n  mobile-android:\n    env:\n      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"1\"\n",
            "jobs:\n  desktop-linux:\n    env:\n      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"\n  mobile-android:\n    steps: []\n",
            "jobs:\n  mobile-android:\n    env:\n      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"\n      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"\n",
            "jobs:\n  mobile-android:\n    steps:\n      - name: too narrow\n        env:\n          DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"\n        run: true\n",
            "jobs:\n  mobile-android:\n    env:\n      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"\n    steps:\n      - name: duplicate override\n        env:\n          DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"\n        run: true\n",
            "jobs:\n  mobile-android:\n    env:\n      COMMENT: |\n        DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"\n    steps: []\n",
        ] {
            assert!(
                validated_mobile_android_job(invalid).is_err(),
                "invalid fixture unexpectedly passed: {invalid}"
            );
        }
    }

    #[test]
    fn desktop_cdp_smokes_use_one_exact_webview2_assigned_port() {
        let valid = "$psi.Environment[\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\"] = \"--remote-debugging-port=0\"\n";
        validate_webview2_cdp_arguments(valid, "valid").expect("valid CDP assignment");

        for invalid in [
            "$psi.Environment[\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\"] = \"--remote-debugging-port=9222\"\n",
            "$psi.Environment[\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\"] = \"--remote-debugging-port=0 --remote-allow-origins=*\"\n",
            "$psi.Environment[\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\"] = \"--remote-debugging-port=0\"\n$psi.Environment[\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\"] = \"--remote-debugging-port=0\"\n",
            "$psi.Environment[\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\"] = \"--remote-debugging-port=0\"\n# --remote-debugging-port=9222\n",
        ] {
            assert!(
                validate_webview2_cdp_arguments(invalid, "invalid").is_err(),
                "invalid CDP assignment unexpectedly passed: {invalid}"
            );
        }
    }
}
