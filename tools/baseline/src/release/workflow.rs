//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

mod aggregate;
mod android_admission;
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
    android_admission::check(root)?;
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
use native_candidate::{
    validate_programmatic_webview2_cdp, validate_webview2_cdp_arguments,
    validated_mobile_android_jobs,
};

#[cfg(test)]
mod tests {
    use super::{
        validate_programmatic_webview2_cdp, validate_webview2_cdp_arguments,
        validated_mobile_android_jobs,
    };

    #[test]
    fn android_candidate_disables_unrelated_linux_host_packaging_check() {
        let valid = r#"jobs:
  mobile-android-apk-build:
    env:
      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: "0"
    steps: []
  mobile-android-arm64-build:
    env:
      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: "0"
    steps: []
"#;
        validated_mobile_android_jobs(valid).expect("valid Android jobs");

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
                validated_mobile_android_jobs(invalid).is_err(),
                "invalid fixture unexpectedly passed: {invalid}"
            );
        }
    }

    #[test]
    fn desktop_cdp_smokes_use_one_exact_programmatic_assigned_port_marker() {
        let valid = "$psi.Environment.Remove(\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\") | Out-Null\n$psi.Environment[\"DEVE_DESKTOP_WEBVIEW2_CDP\"] = \"assigned-loopback\"\n";
        validate_webview2_cdp_arguments(valid, "valid").expect("valid CDP marker");

        for invalid in [
            "$psi.Environment[\"DEVE_DESKTOP_WEBVIEW2_CDP\"] = \"assigned-loopback\"\n",
            "$psi.Environment.Remove(\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\") | Out-Null\n$psi.Environment[\"DEVE_DESKTOP_WEBVIEW2_CDP\"] = \"assigned-loopback \"\n",
            "$psi.Environment.Remove(\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\") | Out-Null\n$psi.Environment[\"DEVE_DESKTOP_WEBVIEW2_CDP\"] = \"assigned-loopback\"\n$psi.Environment[\"DEVE_DESKTOP_WEBVIEW2_CDP\"] = \"assigned-loopback\"\n",
            "$psi.Environment.Remove(\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\") | Out-Null\n$psi.Environment[\"DEVE_DESKTOP_WEBVIEW2_CDP\"] = \"assigned-loopback\"\n# --remote-debugging-port=9222\n",
            "$psi.Environment[\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\"] = \"--remote-debugging-port=0\"\n$psi.Environment[\"DEVE_DESKTOP_WEBVIEW2_CDP\"] = \"assigned-loopback\"\n",
        ] {
            assert!(
                validate_webview2_cdp_arguments(invalid, "invalid").is_err(),
                "invalid CDP assignment unexpectedly passed: {invalid}"
            );
        }
    }

    #[test]
    fn desktop_programmatic_cdp_contract_is_exact_and_rejects_environment_passthrough() {
        let valid = concat!(
            "pub const DEVE_DESKTOP_WEBVIEW2_CDP_ENV: &str = \"DEVE_DESKTOP_WEBVIEW2_CDP\";\n",
            "#[cfg(any(target_os = \"windows\", test))]\n",
            "const DEVE_DESKTOP_WEBVIEW2_CDP_ASSIGNED_LOOPBACK: &str = \"assigned-loopback\";\n",
            "#[cfg(any(target_os = \"windows\", test))]\n",
            "const DEVE_DESKTOP_WEBVIEW2_CDP_BROWSER_ARGS: &str = concat!(\n",
            "    \"--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \",\n",
            "    \"--remote-debugging-port=0\"\n",
            ");\n",
            "    #[cfg(target_os = \"windows\")]\n",
            "    if let Some(arguments) =\n",
            "        desktop_webview2_cdp_browser_arguments(std::env::var_os(DEVE_DESKTOP_WEBVIEW2_CDP_ENV))\n",
            "    {\n",
            "        builder = builder.additional_browser_args(arguments);\n",
            "    }\n",
            "#[cfg(any(target_os = \"windows\", test))]\n",
            "fn desktop_webview2_cdp_browser_arguments(\n",
            "    marker: Option<std::ffi::OsString>,\n",
            ") -> Option<&'static str> {\n",
            "    (marker.as_deref()\n",
            "        == Some(std::ffi::OsStr::new(\n",
            "            DEVE_DESKTOP_WEBVIEW2_CDP_ASSIGNED_LOOPBACK,\n",
            "        )))\n",
            "    .then_some(DEVE_DESKTOP_WEBVIEW2_CDP_BROWSER_ARGS)\n",
            "}\n",
        );
        validate_programmatic_webview2_cdp(valid).expect("valid programmatic CDP contract");

        let extra_argument = valid.replace(
            "\"--remote-debugging-port=0\"",
            "\"--remote-debugging-port=0 --disable-web-security\"",
        );
        let unguarded_builder = valid.replace(
            concat!(
                "    #[cfg(target_os = \"windows\")]\n",
                "    if let Some(arguments) =\n",
                "        desktop_webview2_cdp_browser_arguments(std::env::var_os(DEVE_DESKTOP_WEBVIEW2_CDP_ENV))\n",
                "    {\n",
                "        builder = builder.additional_browser_args(arguments);\n",
                "    }",
            ),
            concat!(
                "    let arguments = DEVE_DESKTOP_WEBVIEW2_CDP_BROWSER_ARGS;\n",
                "    builder = builder.additional_browser_args(arguments);",
            ),
        );
        for invalid in [
            valid.replace("\"assigned-loopback\"", "\"assigned-loopback \""),
            valid.replace(
                "\"--remote-debugging-port=0\"",
                "\"--remote-debugging-port=9222\"",
            ),
            format!("{valid}{valid}"),
            format!("{valid}const LEGACY: &str = \"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\";\n"),
            extra_argument,
            unguarded_builder,
        ] {
            assert!(
                validate_programmatic_webview2_cdp(&invalid).is_err(),
                "invalid programmatic CDP fixture unexpectedly passed"
            );
        }
    }
}
