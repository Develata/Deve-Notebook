//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!   - 18_release#release-versioning
//!
//! Workflow projections of the typed release freeze.

mod promotion;

use super::candidate::{FrozenArtifactRef, fixed_artifacts};
use super::{ReleaseFreeze, read_text};
use anyhow::{Context, Result, ensure};
#[cfg(test)]
pub(super) use promotion::validate_promotion_assets_step;
use std::path::Path;

const CANDIDATE_WORKFLOW: &str = ".github/workflows/release-candidate.yml";
const NATIVE_WORKFLOW: &str = ".github/workflows/release-native.yml";
const PROMOTION_WORKFLOW: &str = ".github/workflows/release.yml";
const FREEZE_VERIFY_CARGO: &str =
    "cargo run --locked --quiet -p deve_baseline -- release-freeze verify-candidate";
const CARGO_AUDIT_VERSION_CHECK: &str =
    r#"run: test "$(cargo-audit --version)" = "cargo-audit 0.22.2""#;

pub(super) fn validate_workflows(root: &Path, registry: &ReleaseFreeze) -> Result<()> {
    let candidate = read_text(root.join(CANDIDATE_WORKFLOW))?;
    let native = read_text(root.join(NATIVE_WORKFLOW))?;
    let promotion = read_text(root.join(PROMOTION_WORKFLOW))?;
    validate_outer_job_budgets(&candidate, &native)?;
    validate_workflow_texts(&candidate, &promotion, registry)
}

pub(super) fn validate_outer_job_budgets(candidate: &str, native: &str) -> Result<()> {
    for (workflow, job, timeout, offset) in [
        (candidate, "docker-linux-amd64:", "timeout-minutes: 360", 3),
        (native, "desktop-windows:", "timeout-minutes: 300", 2),
    ] {
        let lines = active_lines(workflow);
        let job_index = line_index(&lines, job)?;
        ensure!(
            lines.get(job_index + offset) == Some(&timeout),
            "{job} outer timeout must contain the complete serial producer budget"
        );
    }
    Ok(())
}

pub(super) fn validate_workflow_texts(
    candidate: &str,
    promotion: &str,
    registry: &ReleaseFreeze,
) -> Result<()> {
    let cargo_audit = step_block(candidate, "Verify cargo-audit version")?;
    require_exact_line(
        cargo_audit,
        CARGO_AUDIT_VERSION_CHECK,
        "candidate direct cargo-audit version verification",
    )?;
    let candidate_verify = step_block(candidate, "Verify candidate scripts and formatting")?;
    require_exact_line(
        candidate_verify,
        FREEZE_VERIFY_CARGO,
        "candidate freeze verification",
    )?;
    validate_candidate_receipt_producer(
        candidate,
        "Run Remote Import provider and browser producer",
        "docker.remote-import-browser",
        r#"--receipt-dir "$RUNNER_TEMP/deve-acceptance-remote-import""#,
        None,
    )?;
    validate_candidate_receipt_producer(
        candidate,
        "Prove Private Vulnerability Reporting is enabled",
        "github.pvr-enabled",
        r#"--receipt-dir "$RUNNER_TEMP/deve-acceptance-github-pvr""#,
        Some("GH_TOKEN: ${{ github.token }}"),
    )?;
    let docker_build = step_block(candidate, "Build linux/amd64 image once")?;
    require_exact_line(
        docker_build,
        r#"--label "org.opencontainers.image.source=https://github.com/${GITHUB_REPOSITORY}" \"#,
        "candidate Docker source repository label",
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

    promotion::validate(promotion, registry)?;
    Ok(())
}

fn validate_candidate_receipt_producer(
    candidate: &str,
    step_name: &str,
    producer_id: &str,
    receipt_line: &str,
    required_environment: Option<&str>,
) -> Result<()> {
    let step = step_block(candidate, step_name)?;
    require_exact_line(
        step,
        "cargo run --locked --quiet -p deve_baseline -- acceptance-run",
        &format!("{producer_id} candidate runner"),
    )?;
    require_exact_line(
        step,
        "--tier tag-ready",
        &format!("{producer_id} tag-ready tier"),
    )?;
    require_exact_line(
        step,
        &format!("--producer {producer_id}"),
        &format!("{producer_id} producer identity"),
    )?;
    require_exact_line(step, receipt_line, &format!("{producer_id} receipt root"))?;
    if let Some(environment) = required_environment {
        require_exact_line(
            step,
            environment,
            &format!("{producer_id} workflow environment"),
        )?;
    }
    let producer_line = format!("--producer {producer_id}");
    ensure!(
        active_lines(candidate)
            .iter()
            .filter(|line| **line == producer_line.as_str())
            .count()
            == 1,
        "candidate workflow must execute {producer_id} exactly once"
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

pub(super) fn require_exact_line(step: &str, expected: &str, label: &str) -> Result<()> {
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

pub(super) fn line_index(lines: &[&str], expected: &str) -> Result<usize> {
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

pub(super) fn active_lines(step: &str) -> Vec<&str> {
    step.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

pub(super) fn active_lines_owned(step: &str) -> Vec<String> {
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
