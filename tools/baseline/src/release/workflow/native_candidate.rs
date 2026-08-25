//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix
//!   - 18_release#remote-browser-candidate-fixture

use super::text::{
    has_v_tag_trigger, require_exact_mapping_keys, require_ordered_text, require_text,
    yaml_mapping_block,
};
use super::{ANDROID_SIGNING_SECRETS, read_workflow};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

pub(super) fn check(root: &Path) -> Result<()> {
    let native = read_workflow(root, "release-native.yml")?;
    let on = yaml_mapping_block(&native, 0, "on")?;
    let workflow_call = yaml_mapping_block(&on, 2, "workflow_call")?;
    let inputs = yaml_mapping_block(&workflow_call, 4, "inputs")?;
    require_exact_mapping_keys(
        &inputs,
        6,
        &["candidate_head", "version", "web_dist_artifact"],
        "release-native.yml inputs",
    )?;
    let secrets = yaml_mapping_block(&workflow_call, 4, "secrets")?;
    require_exact_mapping_keys(
        &secrets,
        6,
        &ANDROID_SIGNING_SECRETS,
        "release-native.yml secrets",
    )?;
    for secret in ANDROID_SIGNING_SECRETS {
        let declaration = yaml_mapping_block(&secrets, 6, secret)?;
        require_text(
            &declaration,
            "required: true",
            "release-native.yml signing secret",
        )?;
    }
    if has_v_tag_trigger(&native)
        || native.contains("  push:")
        || native.contains("gh release")
        || native.contains("softprops/action-gh-release")
        || native.contains("  publish:")
    {
        bail!("release-baseline-check: release-native.yml must remain build/smoke-only");
    }
    let desktop_windows = yaml_mapping_block(&native, 2, "desktop-windows-build")?;
    require_ordered_text(
        &desktop_windows,
        &[
            "Verify candidate identity and version",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Install native packaging tools",
            "Download immutable exact Web projection",
            "web-dist-artifact.mjs verify",
            "Build exact Windows packages once",
            "Seal and compare Windows package inputs",
            "windows-smoke-input.sha256",
            "windows-candidate-input.sha256",
            "Upload immutable Windows smoke input",
            "Upload exact Windows candidate bytes",
        ],
        "release-native.yml desktop-windows-build job",
    )?;
    let desktop_windows_local = yaml_mapping_block(&native, 2, "desktop-windows-local-smoke")?;
    require_ordered_text(
        &desktop_windows_local,
        &[
            "Download immutable Windows smoke input",
            "Verify immutable Windows smoke input",
            "sha256sum -c windows-smoke-input.sha256",
            "Verify Windows Playwright process adapter",
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/playwright-core-process.test.ps1",
            "pwsh -NoProfile -File scripts/playwright-core-process.test.ps1",
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/webview2-cdp.test.ps1",
            "pwsh -NoProfile -File scripts/webview2-cdp.test.ps1",
            "--producer repo-lifecycle.process-windows",
            "--producer desktop.local-backend",
        ],
        "release-native.yml desktop-windows-local-smoke job",
    )?;
    let desktop_windows_remote = yaml_mapping_block(&native, 2, "desktop-windows-remote-smoke")?;
    require_ordered_text(
        &desktop_windows_remote,
        &[
            "Download immutable Windows smoke input",
            "Verify immutable Windows smoke input",
            "sha256sum -c windows-smoke-input.sha256",
            "Download immutable exact Web projection",
            "Install exact NSIS candidate for RemoteBrowser",
            "Start exact-HEAD RemoteBrowser fixture",
            "--producer desktop.remote-browser",
        ],
        "release-native.yml desktop-windows-remote-smoke job",
    )?;

    let desktop_macos = yaml_mapping_block(&native, 2, "desktop-macos-build")?;
    require_ordered_text(
        &desktop_macos,
        &[
            "Verify candidate identity and version",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Download immutable exact Web projection",
            "web-dist-artifact.mjs verify",
            "Build exact macOS package once",
            "macos-candidate-input.sha256",
            "Seal permission-preserving macOS smoke input",
            "shasum -a 256 bundle.tar",
            "Upload immutable macOS smoke input",
            "Upload exact macOS candidate bytes",
        ],
        "release-native.yml desktop-macos-build job",
    )?;
    let desktop_macos_smoke = yaml_mapping_block(&native, 2, "desktop-macos-smoke")?;
    require_ordered_text(
        &desktop_macos_smoke,
        &[
            "Download immutable macOS smoke input",
            "Verify and extract bounded macOS smoke input",
            "shasum -a 256 -c bundle.tar.sha256",
            "--producer desktop.macos-target-host",
        ],
        "release-native.yml desktop-macos-smoke job",
    )?;

    let (mobile_android_apk, mobile_android_arm64) = validated_mobile_android_jobs(&native)?;
    require_ordered_text(
        &mobile_android_apk,
        &[
            "Verify candidate identity and version",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Download immutable exact Web projection",
            "web-dist-artifact.mjs verify",
            "Build x86 release and debug journey APKs once",
            "Seal Android APK input manifest",
            "Upload immutable Android journey APKs",
        ],
        "release-native.yml mobile-android-apk-build job",
    )?;
    let mobile_android_local = yaml_mapping_block(&native, 2, "mobile-android-local-journey")?;
    require_ordered_text(
        &mobile_android_local,
        &[
            "Download immutable Android journey APKs",
            "Download immutable Android harness programs",
            "sha256sum -c android-harness-input.sha256",
            "android_prebuilt_apk_manifest_verify",
            "--producer android.local-backend",
        ],
        "release-native.yml mobile-android-local-journey job",
    )?;
    let mobile_android_remote = yaml_mapping_block(&native, 2, "mobile-android-remote-journey")?;
    require_ordered_text(
        &mobile_android_remote,
        &[
            "Download immutable Android journey APKs",
            "Download immutable Android harness programs",
            "sha256sum -c android-harness-input.sha256",
            "android_prebuilt_apk_manifest_verify",
            "Prepare exact-HEAD Android RemoteBrowser fixture",
            "--producer android.remote-browser",
        ],
        "release-native.yml mobile-android-remote-journey job",
    )?;
    require_ordered_text(
        &mobile_android_arm64,
        &[
            "Verify candidate identity, version, and signing inputs",
            "Build final ARM64 APK once",
            "Sign and verify exact ARM64 APK",
            "Upload exact signed Android candidate bytes",
        ],
        "release-native.yml mobile-android-arm64-build job",
    )?;
    for required in [
        "--producer desktop.local-backend",
        "--producer desktop.remote-browser",
        "--producer desktop.macos-target-host",
        "--producer android.local-backend",
        "--producer android.remote-browser",
        "Verify Windows Playwright process adapter",
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/playwright-core-process.test.ps1",
        "pwsh -NoProfile -File scripts/playwright-core-process.test.ps1",
        "scripts/playwright-core-process.test.ps1",
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/webview2-cdp.test.ps1",
        "pwsh -NoProfile -File scripts/webview2-cdp.test.ps1",
        "scripts/webview2-cdp.test.ps1",
        "deve-notebook-${VERSION}-macos-${arch}.dmg",
        "--ks-pass env:ANDROID_KEYSTORE_PASSWORD",
        "--key-pass env:ANDROID_KEY_PASSWORD",
        "apksigner\" verify --verbose --print-certs",
        "check-android-apk-signer.sh --print-fingerprint",
        "android-signer-sha256.txt",
    ] {
        require_text(&native, required, "release-native.yml")?;
    }
    for forbidden in [
        "desktop-linux:",
        "mobile-ios:",
        "deve-desktop-linux",
        "deve-mobile-ios",
        "DEVE_DESKTOP_PACKAGE_BUNDLES: deb,appimage",
        "macos-universal",
        "pass:$ANDROID_KEYSTORE_PASSWORD",
        "pass:$ANDROID_KEY_PASSWORD",
    ] {
        if native.contains(forbidden) {
            bail!("release-baseline-check: forbidden first-tag native surface {forbidden}");
        }
    }
    for relative in [
        "scripts/check-desktop-packaged-ui-smoke.ps1",
        "scripts/check-desktop-remote-browser-smoke.ps1",
    ] {
        let path = root.join(relative);
        let script = fs::read_to_string(&path)
            .with_context(|| format!("read WebView2 CDP smoke {}", path.display()))?;
        validate_webview2_cdp_arguments(&script, relative)?;
    }
    let desktop_entry_path = root.join("apps/desktop/src/tauri_entry/mod.rs");
    let desktop_entry = fs::read_to_string(&desktop_entry_path)
        .with_context(|| format!("read Desktop Tauri entry {}", desktop_entry_path.display()))?;
    validate_programmatic_webview2_cdp(&desktop_entry)?;
    Ok(())
}

pub(super) fn validate_webview2_cdp_arguments(script: &str, label: &str) -> Result<()> {
    const LEGACY_KEY: &str = "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS";
    const MARKER_KEY: &str = "DEVE_DESKTOP_WEBVIEW2_CDP";
    const PORT: &str = "--remote-debugging-port=";
    const REMOVE_LEGACY: &str =
        "$psi.Environment.Remove(\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\") | Out-Null";
    const ASSIGNMENT: &str =
        "$psi.Environment[\"DEVE_DESKTOP_WEBVIEW2_CDP\"] = \"assigned-loopback\"";
    if script.lines().filter(|line| *line == REMOVE_LEGACY).count() != 1
        || script.lines().filter(|line| *line == ASSIGNMENT).count() != 1
        || script.matches(LEGACY_KEY).count() != 1
        || script.matches(MARKER_KEY).count() != 1
        || script.matches(PORT).count() != 0
    {
        bail!(
            "release-baseline-check: {label} must clear environment browser arguments and set the exact programmatic WebView2-assigned CDP marker once"
        );
    }
    Ok(())
}

pub(super) fn validate_programmatic_webview2_cdp(source: &str) -> Result<()> {
    const ENV_DEFINITION: &str =
        "pub const DEVE_DESKTOP_WEBVIEW2_CDP_ENV: &str = \"DEVE_DESKTOP_WEBVIEW2_CDP\";";
    const ARGUMENTS_DEFINITION: &str = r#"#[cfg(any(target_os = "windows", test))]
const DEVE_DESKTOP_WEBVIEW2_CDP_ASSIGNED_LOOPBACK: &str = "assigned-loopback";
#[cfg(any(target_os = "windows", test))]
const DEVE_DESKTOP_WEBVIEW2_CDP_BROWSER_ARGS: &str = concat!(
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection ",
    "--remote-debugging-port=0"
);"#;
    const GUARDED_INJECTION: &str = r#"    #[cfg(target_os = "windows")]
    if let Some(arguments) =
        desktop_webview2_cdp_browser_arguments(std::env::var_os(DEVE_DESKTOP_WEBVIEW2_CDP_ENV))
    {
        builder = builder.additional_browser_args(arguments);
    }"#;
    const MARKER_RESOLVER: &str = r#"#[cfg(any(target_os = "windows", test))]
fn desktop_webview2_cdp_browser_arguments(
    marker: Option<std::ffi::OsString>,
) -> Option<&'static str> {
    (marker.as_deref()
        == Some(std::ffi::OsStr::new(
            DEVE_DESKTOP_WEBVIEW2_CDP_ASSIGNED_LOOPBACK,
        )))
    .then_some(DEVE_DESKTOP_WEBVIEW2_CDP_BROWSER_ARGS)
}"#;

    for (label, exact) in [
        ("diagnostic environment marker definition", ENV_DEFINITION),
        ("fixed argument definition", ARGUMENTS_DEFINITION),
        ("marker-guarded builder injection", GUARDED_INJECTION),
        ("exact marker resolver", MARKER_RESOLVER),
    ] {
        if source.matches(exact).count() != 1 {
            bail!(
                "release-baseline-check: Desktop programmatic WebView2 CDP contract must contain exactly one {label}"
            );
        }
    }
    if source.contains("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS")
        || source.matches("--remote-debugging-port=").count() != 1
        || source.matches("additional_browser_args(").count() != 1
    {
        bail!(
            "release-baseline-check: Desktop Tauri entry must expose only the exact marker-guarded programmatic WebView2 CDP arguments"
        );
    }
    Ok(())
}

pub(super) fn validated_mobile_android_jobs(native: &str) -> Result<(String, String)> {
    let apk = yaml_mapping_block(native, 2, "mobile-android-apk-build")?;
    let arm64 = yaml_mapping_block(native, 2, "mobile-android-arm64-build")?;
    const KEY: &str = "DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK:";
    const ASSIGNMENT: &str = "      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"";
    for (name, job) in [
        ("mobile-android-apk-build", &apk),
        ("mobile-android-arm64-build", &arm64),
    ] {
        let job_env = yaml_mapping_block(job, 4, "env")?;
        if job_env.lines().filter(|line| *line == ASSIGNMENT).count() != 1
            || job.matches(KEY).count() != 1
        {
            bail!(
                "release-baseline-check: release-native.yml {name} job-level env must set {KEY} \"0\" exactly once"
            );
        }
    }
    Ok((apk, arm64))
}
