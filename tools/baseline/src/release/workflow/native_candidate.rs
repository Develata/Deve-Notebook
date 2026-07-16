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
        &["candidate_head", "version"],
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
    let desktop_windows = yaml_mapping_block(&native, 2, "desktop-windows")?;
    require_ordered_text(
        &desktop_windows,
        &[
            "Verify candidate identity and version",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Install native packaging tools",
            "Verify Windows Playwright process adapter",
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/playwright-core-process.test.ps1",
            "pwsh -NoProfile -File scripts/playwright-core-process.test.ps1",
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/webview2-cdp.test.ps1",
            "pwsh -NoProfile -File scripts/webview2-cdp.test.ps1",
            "Build exact Web projection",
            "--producer desktop.local-backend",
            "--producer desktop.remote-browser",
        ],
        "release-native.yml desktop-windows job",
    )?;

    let desktop_macos = yaml_mapping_block(&native, 2, "desktop-macos")?;
    require_ordered_text(
        &desktop_macos,
        &[
            "Verify candidate identity and version",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Build exact Web projection",
            "--producer desktop.macos-target-host",
        ],
        "release-native.yml desktop-macos job",
    )?;

    let mobile_android = validated_mobile_android_job(&native)?;
    require_ordered_text(
        &mobile_android,
        &[
            "Verify candidate identity, version, and signing inputs",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Build exact Web projection",
            "--producer android.local-backend",
            "--producer android.remote-browser",
        ],
        "release-native.yml mobile-android job",
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
    Ok(())
}

pub(super) fn validate_webview2_cdp_arguments(script: &str, label: &str) -> Result<()> {
    const KEY: &str = "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS";
    const PORT: &str = "--remote-debugging-port=";
    const ASSIGNMENT: &str = "$psi.Environment[\"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS\"] = \"--remote-debugging-port=0\"";
    if script.lines().filter(|line| *line == ASSIGNMENT).count() != 1
        || script.matches(KEY).count() != 1
        || script.matches(PORT).count() != 1
    {
        bail!(
            "release-baseline-check: {label} must set the exact WebView2-assigned CDP argument once"
        );
    }
    Ok(())
}

pub(super) fn validated_mobile_android_job(native: &str) -> Result<String> {
    let mobile_android = yaml_mapping_block(native, 2, "mobile-android")?;
    let job_env = yaml_mapping_block(&mobile_android, 4, "env")?;
    const KEY: &str = "DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK:";
    const ASSIGNMENT: &str = "      DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK: \"0\"";
    if job_env.lines().filter(|line| *line == ASSIGNMENT).count() != 1
        || mobile_android.matches(KEY).count() != 1
    {
        bail!(
            "release-baseline-check: release-native.yml mobile-android job-level env must set {KEY} \"0\" exactly once"
        );
    }
    Ok(mobile_android)
}
