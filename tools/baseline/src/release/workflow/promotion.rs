//! plan_ref:
//!   - 18_release#release-versioning
//!   - 18_release#artifact-identity-and-integrity

use super::read_workflow;
use super::text::{has_v_tag_trigger, require_ordered_text, require_text, yaml_mapping_block};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

pub(super) fn require_single_tag_entry(root: &Path) -> Result<()> {
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

pub(super) fn check(root: &Path) -> Result<()> {
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
