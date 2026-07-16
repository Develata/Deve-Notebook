//! plan_ref: infra

mod text;

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use text::{
    has_v_tag_trigger, require_exact_mapping_keys, require_ordered_text, require_text,
    yaml_mapping_block,
};

const ANDROID_SIGNING_SECRETS: [&str; 4] = [
    "ANDROID_KEYSTORE_BASE64",
    "ANDROID_KEYSTORE_PASSWORD",
    "ANDROID_KEY_ALIAS",
    "ANDROID_KEY_PASSWORD",
];

pub(super) fn check(root: &Path) -> Result<()> {
    require_single_tag_entry(root)?;
    check_candidate(root)?;
    check_native_candidate(root)?;
    check_aggregate(root)?;
    check_promotion(root)?;
    Ok(())
}

fn require_single_tag_entry(root: &Path) -> Result<()> {
    let workflow_dir = root.join(".github/workflows");
    let mut direct_tag_entries = Vec::new();
    for entry in fs::read_dir(&workflow_dir).context("read .github/workflows")? {
        let path = entry?.path();
        if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("read workflow {}", path.display()))?;
        if has_v_tag_trigger(&content) {
            direct_tag_entries.push(
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
    }
    direct_tag_entries.sort();
    if direct_tag_entries != ["release.yml"] {
        bail!(
            "release-baseline-check: expected release.yml as the only direct v* tag entry, found {:?}",
            direct_tag_entries
        );
    }
    Ok(())
}

fn check_candidate(root: &Path) -> Result<()> {
    let candidate = read_workflow(root, "release-candidate.yml")?;
    require_text(&candidate, "workflow_dispatch:", "release-candidate.yml")?;
    require_text(&candidate, "version:", "release-candidate.yml input")?;
    require_text(
        &candidate,
        "group: release-candidate-${{ github.sha }}-${{ inputs.version }}",
        "release-candidate.yml concurrency",
    )?;
    if has_v_tag_trigger(&candidate) || candidate.contains("  push:") {
        bail!("release-baseline-check: release-candidate.yml must be manual-only");
    }
    let candidate_header = candidate.split("\njobs:").next().unwrap_or(&candidate);
    for forbidden in [
        "artifact-metadata: write",
        "attestations: write",
        "id-token: write",
    ] {
        if candidate_header.contains(forbidden) {
            bail!(
                "release-baseline-check: release-candidate.yml top-level permissions overgrant {forbidden}"
            );
        }
    }

    let validate = yaml_mapping_block(&candidate, 2, "validate")?;
    require_ordered_text(
        &validate,
        &[
            "Checkout exact dispatch commit",
            "ref: ${{ github.sha }}",
            "Lock candidate version and commit",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "scripts/check-release-version-match.sh \"v$VERSION\"",
            "Install Linux native preflight dependencies",
            "libwebkit2gtk-4.1-dev",
            "libgtk-3-dev",
            "librsvg2-dev",
            "libayatana-appindicator3-dev",
            "cargo fmt --all -- --check",
            "Install Web projection build tool",
            "DEVE_NATIVE_INSTALL_TAURI_CLI: \"0\"",
            "scripts/install-native-target-host-tools.sh",
            "Build exact Web projection",
            "scripts/build-web-dist-ci.sh",
            "cargo run --locked --quiet -p deve_baseline -- all",
            "cargo run --locked --quiet -p deve_baseline -- full",
            "cargo clippy --locked --all-targets -- -D warnings",
            "cargo test --locked",
        ],
        "release-candidate.yml validate job",
    )?;

    let docker = yaml_mapping_block(&candidate, 2, "docker-linux-amd64")?;
    require_ordered_text(
        &docker,
        &[
            "Verify exact checkout",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Build linux/amd64 image once",
            "--platform linux/amd64",
            "docker image inspect --format '{{.Os}}/{{.Architecture}}'",
            "--producer docker.multiclient-product",
            "--producer docker.p2p-gap-recovery",
            "--producer security.tag-ready-audit",
            "docker save --output",
            "docker image rm",
            "docker load --input",
            "Generate exact Docker image SPDX 2.3 SBOM",
            "deve-docker-candidate-${{ github.sha }}",
        ],
        "release-candidate.yml Docker job",
    )?;
    if docker.matches("docker buildx build").count() != 1 {
        bail!("release-baseline-check: candidate Docker image must be built exactly once");
    }

    let native_call = yaml_mapping_block(&candidate, 2, "native")?;
    require_text(
        &native_call,
        "uses: ./.github/workflows/release-native.yml",
        "release-candidate.yml native call",
    )?;
    require_text(
        &native_call,
        "candidate_head: ${{ github.sha }}",
        "release-candidate.yml native call",
    )?;
    if native_call.contains("secrets: inherit") {
        bail!("release-baseline-check: candidate must not inherit all native secrets");
    }
    let secret_map = yaml_mapping_block(&native_call, 4, "secrets")?;
    require_exact_mapping_keys(
        &secret_map,
        6,
        &ANDROID_SIGNING_SECRETS,
        "release-candidate.yml native secret map",
    )?;

    let assemble = yaml_mapping_block(&candidate, 2, "assemble")?;
    require_ordered_text(
        &assemble,
        &[
            "Checkout exact candidate",
            "Lock immutable candidate attempt",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Generate exact source SPDX 2.3 SBOM before artifact download",
            "Download exact Windows candidate",
            "Download exact macOS candidate",
            "Download exact Android candidate",
            "Download exact Docker candidate",
            "Materialize a strict candidate tree",
            "require_count windows",
            "require_count macos",
            "require_count android",
            "require_count docker",
            "actions/attest@a1948c3f048ba23858d222213b7c278aabede763",
            "sbom-path:",
            "create-storage-record: false",
            "Seal the returned attestation bundles",
            "--provenance-bundle",
            "--docker-sbom-bundle",
            "release-candidate assemble",
            "release-candidate verify",
            "--producer release.candidate-bundle",
            "deve-release-candidate-${{ github.sha }}",
        ],
        "release-candidate.yml assemble job",
    )?;
    for permission in [
        "artifact-metadata: write",
        "attestations: write",
        "id-token: write",
    ] {
        require_text(
            &assemble,
            permission,
            "release-candidate.yml assemble permissions",
        )?;
    }
    Ok(())
}

fn check_native_candidate(root: &Path) -> Result<()> {
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

fn validate_webview2_cdp_arguments(script: &str, label: &str) -> Result<()> {
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

fn validated_mobile_android_job(native: &str) -> Result<String> {
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

fn check_aggregate(root: &Path) -> Result<()> {
    let aggregate = read_workflow(root, "acceptance-aggregate.yml")?;
    require_ordered_text(
        &aggregate,
        &[
            "receipt_run_ids:",
            "exactly one release-candidate run ID is required",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            ".github/workflows/release-candidate.yml",
            "candidate run attempt must be 1; dispatch a fresh release candidate",
            "deve-release-candidate-$GITHUB_SHA",
            "deve-acceptance-receipts-*",
            "Verify sealed artifacts and GitHub attestations",
            "candidate manifest run attempt does not match Actions metadata",
            "provenance-attestation",
            "docker-sbom-attestation",
            "scripts/check-release-candidate-bundle.sh",
            "acceptance-collect",
            "acceptance-matrix",
            "--tag-ready",
            "deve-release-sealed-${{ github.sha }}",
        ],
        "acceptance-aggregate.yml",
    )?;
    if has_v_tag_trigger(&aggregate) || aggregate.contains("  push:") {
        bail!("release-baseline-check: acceptance aggregate must be manual-only");
    }
    require_text(
        &aggregate,
        "persist-credentials: false",
        "acceptance aggregate checkout",
    )?;
    Ok(())
}

fn check_promotion(root: &Path) -> Result<()> {
    let release = read_workflow(root, "release.yml")?;
    let promote = yaml_mapping_block(&release, 2, "promote-sealed-candidate")?;
    require_ordered_text(
        &promote,
        &[
            "Validate SemVer tag before checkout",
            "invalid SemVer release tag",
            "uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0",
            "Bind annotated tag to one aggregate run",
            "git rev-parse \"${GITHUB_REF}^{}\"",
            "Deve-Acceptance-Aggregate-Run:",
            ".github/workflows/acceptance-aggregate.yml",
            "bound aggregate run must be immutable attempt 1",
            "deve-release-sealed-$GITHUB_SHA",
            "Verify candidate bytes, tag version, attestations, and receipts",
            "provenance-attestation",
            "docker-sbom-attestation",
            "scripts/check-release-candidate-bundle.sh",
            "acceptance-matrix",
            "--tag-ready",
            "Create or validate draft and upload unchanged assets",
            "scripts/check-release-tag-binding.sh",
            "scripts/probe-release-remote.sh",
            "gh release upload",
            "docker load --input",
            "release-version-order",
            "registry_version=\"${version/+/_build_}\"",
            "docker push \"$VERSION_TAG\"",
            "docker buildx imagetools inspect",
            "actions/attest@a1948c3f048ba23858d222213b7c278aabede763",
            "create-storage-record: false",
            "Publish verified GitHub Release",
            "gh release edit",
        ],
        "release.yml promotion job",
    )?;
    require_text(
        &release,
        "group: release-promotion-${{ github.repository }}",
        "release.yml promotion concurrency",
    )?;
    require_text(
        &release,
        "persist-credentials: false",
        "release.yml checkout",
    )?;
    require_text(
        &release,
        "already_published",
        "published-release idempotent recovery",
    )?;
    for forbidden in [
        "docker build ",
        "docker buildx build",
        "docker/build-push-action",
        "cargo tauri",
        "cargo test",
        "cargo clippy",
        "scripts/build-web-dist-ci.sh",
        "uses: ./.github/workflows/release-native.yml",
        "secrets: inherit",
        "runs?head_sha=$GITHUB_SHA",
    ] {
        if promote.contains(forbidden) {
            bail!(
                "release-baseline-check: tag promotion contains forbidden rebuild/latest-selection token {forbidden}"
            );
        }
    }
    if release.matches("gh release edit").count() != 1 {
        bail!("release-baseline-check: release.yml must publish the verified draft exactly once");
    }
    Ok(())
}

fn read_workflow(root: &Path, name: &str) -> Result<String> {
    let path = root.join(".github/workflows").join(name);
    fs::read_to_string(&path).with_context(|| format!("read workflow {}", path.display()))
}

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
