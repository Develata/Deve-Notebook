//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!   - 18_release#release-versioning
//!
//! Promotion-only projection checks. Candidate materialization stays in the
//! parent module so the public mutation boundary remains explicit.

use super::super::ReleaseFreeze;
use super::{active_lines, active_lines_owned, line_index, require_exact_line, step_block};
use anyhow::{Context, Result, ensure};

const FREEZE_VERIFY_BINARY: &str = "target/debug/deve_baseline release-freeze verify-candidate";
const RELEASE_NOTES_BINARY: &str =
    r#"target/debug/deve_baseline release-freeze release-notes >"$release_notes""#;
const RELEASE_CHANNEL_LINE: &str =
    r#"release_channel="$(jq -er .release.channel docs/registry/release-freeze.json)""#;

pub(super) fn validate(promotion: &str, registry: &ReleaseFreeze) -> Result<()> {
    let promotion_verify = step_block(
        promotion,
        "Verify candidate bytes, tag version, attestations, and receipts",
    )?;
    require_exact_line(
        promotion_verify,
        FREEZE_VERIFY_BINARY,
        "promotion freeze verification",
    )?;
    let assets = step_block(promotion, "Stage unchanged public release assets")?;
    validate_promotion_assets_step(assets, registry)?;
    validate_upload_step(promotion)?;
    validate_release_notes_projection(promotion)?;
    validate_release_channel_projection(promotion)?;
    validate_public_ghcr_projection(promotion)
}

pub(crate) fn validate_promotion_assets_step(step: &str, registry: &ReleaseFreeze) -> Result<()> {
    let controls = &registry.controls;
    let expected = vec![
        "- name: Stage unchanged public release assets".to_owned(),
        "id: assets".to_owned(),
        "shell: bash".to_owned(),
        "run: |".to_owned(),
        "set -euo pipefail".to_owned(),
        r#"candidate="$DEVE_SEALED_ROOT/candidate""#.to_owned(),
        r#"manifest="$candidate/release-candidate.json""#.to_owned(),
        r#"asset_list="$RUNNER_TEMP/release-assets.txt""#.to_owned(),
        r#"name_list="$RUNNER_TEMP/release-asset-names.txt""#.to_owned(),
        r#"jq -er '.artifacts[] | select(.public == true) | .path' "$manifest" >"$asset_list""#
            .to_owned(),
        format!(
            "printf '%s\\n' {} {} >>\"$asset_list\"",
            controls.release_candidate.path, controls.public_checksums.path
        ),
        "while IFS= read -r relative; do".to_owned(),
        r#"[[ -f "$candidate/$relative" ]] || { echo "missing release asset: $relative" >&2; exit 1; }"#
            .to_owned(),
        r#"basename "$relative""#.to_owned(),
        r#"done <"$asset_list" | sort >"$name_list""#.to_owned(),
        r#"[[ "$(sort -u "$name_list" | wc -l)" -eq "$(wc -l <"$name_list")" ]] || {"#
            .to_owned(),
        r#"echo "release assets contain duplicate basenames" >&2"#.to_owned(),
        "exit 1".to_owned(),
        "}".to_owned(),
        r#"printf 'asset_list=%s\n' "$asset_list" >>"$GITHUB_OUTPUT""#.to_owned(),
        r#"printf 'name_list=%s\n' "$name_list" >>"$GITHUB_OUTPUT""#.to_owned(),
    ];
    ensure!(
        active_lines_owned(step) == expected,
        "promotion asset selection must be the exact frozen manifest-public/control projection"
    );
    ensure!(
        !step.contains(&controls.candidate_checksums.path),
        "promotion must not expose candidate-internal checksums"
    );
    Ok(())
}

fn validate_upload_step(promotion: &str) -> Result<()> {
    let upload = step_block(
        promotion,
        "Create or validate draft and upload unchanged assets",
    )?;
    require_exact_line(
        upload,
        "export DEVE_RELEASE_ATTESTATION_VERIFY_REQUIRED=1",
        "pre-upload attestation verification",
    )?;
    require_exact_line(
        upload,
        "bash scripts/check-release-candidate-bundle.sh",
        "immediate pre-upload candidate verification",
    )?;
    require_exact_line(
        upload,
        r#"while IFS= read -r relative; do upload+=("$candidate/$relative"); done <"$ASSET_LIST""#,
        "release upload list construction",
    )?;
    require_exact_line(
        upload,
        r#"gh release upload "$GITHUB_REF_NAME" --clobber "${upload[@]}""#,
        "release upload command",
    )?;
    let lines = active_lines(upload);
    let verify = line_index(&lines, "export DEVE_RELEASE_ATTESTATION_VERIFY_REQUIRED=1")?;
    ensure!(
        lines.get(verify + 1) == Some(&"bash scripts/check-release-candidate-bundle.sh")
            && lines.get(verify + 2) == Some(&"upload=()")
            && lines.get(verify + 3)
                == Some(
                    &r#"while IFS= read -r relative; do upload+=("$candidate/$relative"); done <"$ASSET_LIST""#,
                )
            && lines.get(verify + 4)
                == Some(&r#"gh release upload "$GITHUB_REF_NAME" --clobber "${upload[@]}""#),
        "candidate verification must be immediately followed by exact asset-list upload"
    );
    ensure!(
        lines
            .iter()
            .filter(|line| line.contains("upload+=("))
            .count()
            == 1
            && lines
                .iter()
                .filter(|line| line.contains("gh release upload"))
                .count()
                == 1,
        "release upload step contains an additional asset injection path"
    );
    ensure!(
        lines
            .iter()
            .filter(|line| line.contains("ASSET_LIST"))
            .count()
            == 3,
        "release upload step mutates or rebinds the frozen asset list"
    );
    let workflow_lines = active_lines(promotion);
    ensure!(
        workflow_lines
            .iter()
            .filter(|line| line.contains("gh release upload"))
            .count()
            == 1
            && workflow_lines
                .iter()
                .filter(|line| line.contains("gh release edit"))
                .count()
                == 1,
        "promotion workflow must contain exactly one release upload and edit path"
    );
    Ok(())
}

fn validate_release_notes_projection(promotion: &str) -> Result<()> {
    let upload = step_block(
        promotion,
        "Create or validate draft and upload unchanged assets",
    )?;
    for (line, label) in [
        (
            r#"release_notes="$RUNNER_TEMP/release-notes.md""#,
            "release notes path",
        ),
        (RELEASE_NOTES_BINARY, "release notes rendering"),
        (
            r#"sed 's/\r$//' "$release_notes" >"$RUNNER_TEMP/release-notes.canonical.md""#,
            "release notes line-ending normalization",
        ),
        (
            r#"jq -ej '.body | if type == "string" then . else error("invalid release body") end' \"#,
            "existing release notes exact extraction",
        ),
        (
            r#"sed 's/\r$//' "$RUNNER_TEMP/existing-release-notes.raw.md" \"#,
            "existing release notes line-ending normalization",
        ),
        (
            r#">"$RUNNER_TEMP/existing-release-notes.canonical.md""#,
            "existing release notes canonical output",
        ),
        (
            r#"diff -u "$RUNNER_TEMP/release-notes.canonical.md" \"#,
            "existing release notes identity",
        ),
        (
            r#""$RUNNER_TEMP/existing-release-notes.canonical.md""#,
            "existing release notes canonical diff input",
        ),
        (
            r#"--notes-file "$release_notes" \"#,
            "draft release notes input",
        ),
        (
            r#"gh release create "$GITHUB_REF_NAME" --verify-tag --draft --latest=false \"#,
            "draft release latest classification",
        ),
    ] {
        require_exact_line(upload, line, label)?;
    }
    ensure!(
        !active_lines(upload)
            .iter()
            .any(|line| line.contains("--generate-notes")),
        "release workflow must use frozen CHANGELOG notes instead of generated notes"
    );

    let publish = step_block(promotion, "Publish verified GitHub Release")?;
    require_exact_line(
        publish,
        r#"--notes-file "$RUNNER_TEMP/release-notes.md" "${args[@]}""#,
        "published release notes input",
    )
}

fn validate_public_ghcr_projection(promotion: &str) -> Result<()> {
    let registry_step = "Promote exact Docker bytes and verify remote digest";
    let publish_step = "Publish verified GitHub Release";
    let registry = step_block(promotion, registry_step)?;
    for (line, label) in [
        (
            r#"anonymous_config="$RUNNER_TEMP/deve-ghcr-anonymous""#,
            "anonymous Docker config path",
        ),
        (
            r#"mkdir "$anonymous_config""#,
            "fresh anonymous Docker config",
        ),
        (
            r#"DOCKER_CONFIG="$anonymous_config" docker pull "$VERSION_TAG" >/dev/null"#,
            "anonymous GHCR pull",
        ),
    ] {
        require_exact_line(registry, line, label)?;
    }
    let registry_lines = active_lines(registry);
    let anonymous_pull = line_index(
        &registry_lines,
        r#"DOCKER_CONFIG="$anonymous_config" docker pull "$VERSION_TAG" >/dev/null"#,
    )?;
    ensure!(
        registry_lines.get(anonymous_pull + 1)
            == Some(
                &r#"[[ "$(docker image inspect --format '{{.Id}}' "$VERSION_TAG")" == "$IMAGE_ID" ]] || {"#,
            ),
        "anonymous GHCR pull must be followed by exact image identity validation"
    );
    let registry_index = promotion
        .find(&format!("- name: {registry_step}"))
        .context("promotion workflow is missing the registry step")?;
    let publish_index = promotion
        .find(&format!("- name: {publish_step}"))
        .context("promotion workflow is missing the publish step")?;
    ensure!(
        registry_index < publish_index,
        "anonymous GHCR verification must precede GitHub Release publication"
    );
    Ok(())
}

fn validate_release_channel_projection(promotion: &str) -> Result<()> {
    let existing = step_block(
        promotion,
        "Create or validate draft and upload unchanged assets",
    )?;
    let docker = step_block(promotion, "Load and validate sealed Docker archive")?;
    let publish = step_block(promotion, "Publish verified GitHub Release")?;
    for (label, step) in [
        ("existing release classification", existing),
        ("Docker latest classification", docker),
        ("GitHub Release classification", publish),
    ] {
        require_exact_line(step, RELEASE_CHANNEL_LINE, label)?;
    }
    for expected in [
        "public-preview) expected_prerelease=true ;;",
        "stable) expected_prerelease=false ;;",
    ] {
        require_exact_line(existing, expected, "existing release channel case")?;
    }
    require_exact_line(
        docker,
        r#"if [[ "$release_channel" == stable ]]; then"#,
        "stable-only latest gate",
    )?;
    require_exact_line(
        docker,
        r#"elif [[ "$release_channel" != public-preview ]]; then"#,
        "public-preview no-latest gate",
    )?;
    for expected in [
        "public-preview) args+=(--prerelease --latest=false) ;;",
        "stable) args+=(--prerelease=false --latest) ;;",
    ] {
        require_exact_line(publish, expected, "published release channel case")?;
    }
    Ok(())
}
