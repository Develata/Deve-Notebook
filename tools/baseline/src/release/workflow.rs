//! plan_ref: infra

mod text;

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use text::{
    has_v_tag_trigger, require_exact_mapping_keys, require_ordered_text, require_text,
    yaml_mapping_block,
};

pub(super) fn check(root: &Path) -> Result<()> {
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

    let release = read_workflow(root, "release.yml")?;
    let test = yaml_mapping_block(&release, 2, "test")?;
    require_ordered_text(
        &test,
        &[
            "Validate SemVer release tag",
            "invalid SemVer release tag",
            "uses: actions/checkout@v6",
        ],
        "release.yml test job",
    )?;

    let native_call = yaml_mapping_block(&release, 2, "native")?;
    require_text(&native_call, "needs: docker", "release.yml native job")?;
    require_text(
        &native_call,
        "uses: ./.github/workflows/release-native.yml",
        "release.yml native job",
    )?;
    if native_call.contains("secrets: inherit") {
        bail!("release-baseline-check: reusable native workflow must not inherit all secrets");
    }
    let native_secret_map = yaml_mapping_block(&native_call, 4, "secrets")?;
    require_exact_mapping_keys(
        &native_secret_map,
        6,
        &ANDROID_SIGNING_SECRETS,
        "release.yml native secret map",
    )?;
    for secret in ANDROID_SIGNING_SECRETS {
        let mapping = format!("{secret}: ${{{{ secrets.{secret} }}}}");
        require_text(
            &native_secret_map,
            &mapping,
            "release.yml native secret map",
        )?;
    }

    let native = read_workflow(root, "release-native.yml")?;
    let on = yaml_mapping_block(&native, 0, "on")?;
    let workflow_call = yaml_mapping_block(&on, 2, "workflow_call")?;
    let declared_secrets = yaml_mapping_block(&workflow_call, 4, "secrets")?;
    require_exact_mapping_keys(
        &declared_secrets,
        6,
        &ANDROID_SIGNING_SECRETS,
        "release-native.yml declared secrets",
    )?;
    if native.lines().any(|line| line == "  push:") || has_v_tag_trigger(&native) {
        bail!("release-baseline-check: release-native.yml must not have a direct push/tag trigger");
    }
    for secret in ANDROID_SIGNING_SECRETS {
        let declaration = yaml_mapping_block(&declared_secrets, 6, secret)?;
        require_text(
            &declaration,
            "required: false",
            "release-native.yml declared secret",
        )?;
    }

    let publish = yaml_mapping_block(&native, 2, "publish")?;
    for required in [
        "needs: [desktop-windows, mobile-android, desktop-macos]",
        "name: deve-desktop-windows",
        "name: deve-mobile-android",
        "name: deve-desktop-macos",
        "Validate native artifact manifest",
        "expected exactly four downloaded files",
        "Require absent or draft GitHub Release",
        "existing GitHub Release must be draft before asset upload",
        "draft: true",
        "diff -u native-asset-manifest.txt uploaded-asset-manifest.txt",
        "gh release edit \"$GITHUB_REF_NAME\"",
    ] {
        require_text(&publish, required, "release-native.yml publish job")?;
    }
    require_ordered_text(
        &publish,
        &[
            "Download Windows artifacts",
            "Download Android artifact",
            "Download macOS artifact",
            "Validate native artifact manifest",
            "expected exactly four downloaded files",
            "Require absent or draft GitHub Release",
            "existing GitHub Release must be draft before asset upload",
            "Upload validated assets to a draft GitHub Release",
            "draft: true",
            "Verify draft assets and publish GitHub Release",
            "release must remain draft before asset verification",
            "diff -u native-asset-manifest.txt uploaded-asset-manifest.txt",
            "gh release edit \"$GITHUB_REF_NAME\"",
            "diff -u native-asset-manifest.txt published-asset-manifest.txt",
        ],
        "release-native.yml publish job",
    )?;

    if publish.matches("uses: actions/download-artifact@").count() != 3 {
        bail!(
            "release-baseline-check: release-native.yml publish must download exactly three allowlisted artifact containers"
        );
    }
    if publish
        .matches("gh release edit \"$GITHUB_REF_NAME\"")
        .count()
        != 1
    {
        bail!(
            "release-baseline-check: release-native.yml publish must make the draft public exactly once"
        );
    }

    let release_action = "uses: softprops/action-gh-release@";
    if native.matches(release_action).count() != 1 || !publish.contains(release_action) {
        bail!(
            "release-baseline-check: release-native.yml must call action-gh-release exactly once inside publish"
        );
    }
    for build_job in ["desktop-windows", "mobile-android", "desktop-macos"] {
        let block = yaml_mapping_block(&native, 2, build_job)?;
        if block.contains(release_action) {
            bail!(
                "release-baseline-check: native build job {build_job} must not publish a GitHub Release"
            );
        }
    }

    Ok(())
}

const ANDROID_SIGNING_SECRETS: [&str; 4] = [
    "ANDROID_KEYSTORE_BASE64",
    "ANDROID_KEYSTORE_PASSWORD",
    "ANDROID_KEY_ALIAS",
    "ANDROID_KEY_PASSWORD",
];

fn read_workflow(root: &Path, name: &str) -> Result<String> {
    let path = root.join(".github/workflows").join(name);
    fs::read_to_string(&path).with_context(|| format!("read workflow {}", path.display()))
}
