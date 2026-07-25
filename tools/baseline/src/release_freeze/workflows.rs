//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!   - 18_release#release-versioning
//!
//! Workflow projections of the typed release freeze.

use super::candidate::{FrozenArtifactRef, fixed_artifacts};
use super::{ReleaseFreeze, read_text};
use anyhow::{Context, Result, ensure};
use std::path::Path;

const CANDIDATE_WORKFLOW: &str = ".github/workflows/release-candidate.yml";
const PROMOTION_WORKFLOW: &str = ".github/workflows/release.yml";
const FREEZE_VERIFY_CARGO: &str =
    "cargo run --locked --quiet -p deve_baseline -- release-freeze verify";
const FREEZE_VERIFY_BINARY: &str = "target/debug/deve_baseline release-freeze verify";
const RELEASE_CHANNEL_LINE: &str =
    r#"release_channel="$(jq -er .release.channel docs/registry/release-freeze.json)""#;

pub(super) fn validate_workflows(root: &Path, registry: &ReleaseFreeze) -> Result<()> {
    let candidate = read_text(root.join(CANDIDATE_WORKFLOW))?;
    let promotion = read_text(root.join(PROMOTION_WORKFLOW))?;
    validate_workflow_texts(&candidate, &promotion, registry)
}

pub(super) fn validate_workflow_texts(
    candidate: &str,
    promotion: &str,
    registry: &ReleaseFreeze,
) -> Result<()> {
    let candidate_verify = step_block(candidate, "Verify candidate scripts and formatting")?;
    require_exact_line(
        candidate_verify,
        FREEZE_VERIFY_CARGO,
        "candidate freeze verification",
    )?;
    let materialize = step_block(candidate, "Materialize a strict candidate tree")?;
    let assemble = step_block(
        candidate,
        "Assemble and independently verify canonical candidate controls",
    )?;
    let attestations = step_block(candidate, "Seal the returned attestation bundles")?;

    for artifact in fixed_artifacts(registry) {
        validate_candidate_artifact_lines(artifact, materialize, attestations, assemble)?;
    }
    require_exact_line(
        materialize,
        "[[ ${#macos_matches[@]} -eq 1 ]]",
        "macOS one-of count",
    )?;
    let macos = &registry.artifacts.macos_host_dmg.one_of;
    let x64 = macos
        .iter()
        .find(|path| path.ends_with("-macos-x64.dmg"))
        .map(|path| path.replace("{version}", "${VERSION}"))
        .context("release freeze is missing macOS x64 choice")?;
    let arm64 = macos
        .iter()
        .find(|path| path.ends_with("-macos-arm64.dmg"))
        .map(|path| path.replace("{version}", "${VERSION}"))
        .context("release freeze is missing macOS arm64 choice")?;
    require_exact_line(
        materialize,
        &format!(r#""{}"|"{}") ;;"#, basename(&x64), basename(&arm64)),
        "macOS executable host-architecture allowlist",
    )?;
    require_exact_line(
        assemble,
        r#"--macos-dmg "$macos_path""#,
        "macOS sealed assembler input",
    )?;
    for forbidden in [".AppImage", ".deb", ".ipa", ".rpm", "macos-universal"] {
        ensure!(
            active_lines(materialize)
                .into_iter()
                .chain(active_lines(assemble))
                .all(|line| !line.contains(forbidden)),
            "candidate tree admits unfrozen artifact marker {forbidden}"
        );
    }

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
    validate_release_channel_projection(promotion)?;
    Ok(())
}

pub(super) fn validate_promotion_assets_step(step: &str, registry: &ReleaseFreeze) -> Result<()> {
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

fn validate_candidate_artifact_lines(
    artifact: FrozenArtifactRef<'_>,
    materialize: &str,
    attestations: &str,
    assemble: &str,
) -> Result<()> {
    let shell_path = artifact.path.replace("{version}", "${VERSION}");
    let owner = match artifact.role {
        "provenance-attestation" | "docker-sbom-attestation" => attestations,
        _ => materialize,
    };
    let owner_prefix = match artifact.role {
        "source-spdx" | "provenance-attestation" | "docker-sbom-attestation" => "cp -- ",
        _ => "require_one ",
    };
    let destination = format!("\"$CANDIDATE_DIR/{shell_path}\"");
    let owner_matches = active_lines(owner)
        .into_iter()
        .filter(|line| line.starts_with(owner_prefix) && line.contains(&destination))
        .count();
    ensure!(
        owner_matches == 1,
        "{CANDIDATE_WORKFLOW} must actively materialize frozen {} exactly once",
        artifact.label
    );
    require_exact_line(
        assemble,
        &format!(r#"{} "{shell_path}""#, role_flag(artifact.role)),
        &format!("{} assembler input", artifact.label),
    )
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
        "public-preview) args+=(--prerelease) ;;",
        "stable) args+=(--prerelease=false --latest) ;;",
    ] {
        require_exact_line(publish, expected, "published release channel case")?;
    }
    Ok(())
}

fn require_exact_line(step: &str, expected: &str, label: &str) -> Result<()> {
    let matches = active_lines(step)
        .into_iter()
        .filter(|line| *line == expected)
        .count();
    ensure!(
        matches == 1,
        "{label} must occur as exactly one active workflow line"
    );
    Ok(())
}

fn line_index(lines: &[&str], expected: &str) -> Result<usize> {
    lines
        .iter()
        .position(|line| *line == expected)
        .with_context(|| format!("active workflow line not found: {expected}"))
}

pub(super) fn step_block<'a>(workflow: &'a str, name: &str) -> Result<&'a str> {
    let marker = format!("- name: {name}");
    let start = workflow
        .find(&marker)
        .with_context(|| format!("workflow step not found: {name}"))?;
    let rest = &workflow[start + marker.len()..];
    let end = rest.find("\n      - name: ").unwrap_or(rest.len());
    Ok(&workflow[start..start + marker.len() + end])
}

fn active_lines(step: &str) -> Vec<&str> {
    step.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn active_lines_owned(step: &str) -> Vec<String> {
    active_lines(step).into_iter().map(str::to_owned).collect()
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or_default()
}

fn role_flag(role: &str) -> &'static str {
    match role {
        "windows-msi" => "--windows-msi",
        "windows-nsis" => "--windows-nsis",
        "android-arm64-apk" => "--android-apk",
        "docker-linux-amd64-archive" => "--docker-archive",
        "source-spdx" => "--source-sbom",
        "image-spdx" => "--image-sbom",
        "provenance-attestation" => "--provenance-bundle",
        "docker-sbom-attestation" => "--docker-sbom-bundle",
        _ => unreachable!("fixed release role"),
    }
}
