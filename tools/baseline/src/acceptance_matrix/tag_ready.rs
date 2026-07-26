//! Fresh receipt validation for first-tag requirements.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix
use super::execution_group::validate_execution_groups;
use super::model::MatrixRow;
use super::producer::artifact_reader::{ReceiptArtifactBudget, ReceiptArtifactRoot};
use super::producer::{ReceiptProducerBinding, receipt_bindings};
use super::receipt::Receipt;
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Utc};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

mod android;
mod desktop;
mod repo_lifecycle;

use android::validate_android_claims;
use desktop::validate_desktop_claims;

#[derive(Clone, Debug)]
struct ReceiptRecord {
    relative_path: String,
    receipt: Receipt,
}

pub(super) fn validate(root: &Path, rows: &[MatrixRow], receipt_dir: &Path) -> Result<()> {
    let head = git_output(root, ["rev-parse", "HEAD"])?;
    if !git_status(root)?.is_empty() {
        bail!("acceptance-matrix tag-ready: current worktree is dirty");
    }
    let receipts = read_receipts(receipt_dir)?;
    let bindings = receipt_bindings(root, rows)?;
    let mut accepted_gaps = crate::release_freeze::accepted_gap_bindings(root)?;
    let mut accepted_gap_ids = BTreeSet::new();
    let mut blockers = Vec::new();
    if let Err(error) = validate_docker_candidate_identity(rows, &receipts) {
        blockers.push(format!("Docker candidate identity: {error}"));
    }
    for row in rows
        .iter()
        .filter(|row| row.gate == "tag-ready" && row.requirement == "required")
    {
        if row.evidence_kind == "gap" {
            if let Some(accepted_gap_id) = consume_accepted_gap(row, &mut accepted_gaps) {
                accepted_gap_ids.insert(accepted_gap_id);
            } else {
                blockers.push(format!(
                    "{} remains an explicit gap: {}",
                    row.requirement_id, row.note
                ));
            }
            continue;
        }
        if row.freshness == "source-bound"
            && matches!(
                row.evidence_kind.as_str(),
                "source-ref" | "document" | "test" | "script"
            )
        {
            continue;
        }
        let Some(record) = receipts.get(&row.evidence_id) else {
            blockers.push(format!(
                "{} missing receipt {}",
                row.requirement_id, row.evidence_id
            ));
            continue;
        };
        let Some(binding) = bindings.get(&row.evidence_id) else {
            blockers.push(format!(
                "{} receipt {} has no producer binding",
                row.requirement_id, row.evidence_id
            ));
            continue;
        };
        if let Err(error) = validate_receipt(record, row, binding, &head, Utc::now()) {
            blockers.push(format!("{}: {error}", row.requirement_id));
        }
    }
    if let Err(error) = crate::release_freeze::reject_unconsumed(&accepted_gaps) {
        blockers.push(error.to_string());
    }
    if blockers.is_empty() {
        if accepted_gap_ids.is_empty() {
            println!("acceptance-matrix tag-ready: ok");
        } else {
            println!(
                "acceptance-matrix tag-ready: ok with accepted known limitation(s): {}",
                accepted_gap_ids.into_iter().collect::<Vec<_>>().join(",")
            );
        }
        Ok(())
    } else {
        bail!(
            "acceptance-matrix tag-ready blocked by {} requirement(s):\n- {}",
            blockers.len(),
            blockers.join("\n- ")
        )
    }
}

fn consume_accepted_gap(
    row: &MatrixRow,
    accepted_gaps: &mut crate::release_freeze::AcceptedGapBindings,
) -> Option<String> {
    accepted_gaps.remove(&(row.requirement_id.clone(), row.evidence_id.clone()))
}

fn validate_receipt(
    record: &ReceiptRecord,
    row: &MatrixRow,
    binding: &ReceiptProducerBinding,
    head: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let receipt = &record.receipt;
    validate_producer_binding(receipt, row, binding)?;
    if receipt.status != "passed" {
        bail!("receipt status is {}", receipt.status);
    }
    if receipt.dirty_before || receipt.dirty_after {
        bail!("receipt command did not preserve a clean worktree");
    }
    if receipt.head != head {
        bail!(
            "receipt HEAD {} does not match current HEAD {head}",
            receipt.head
        );
    }
    if receipt.head_after.as_deref() != Some(head) {
        bail!("receipt HEAD changed or was unavailable after command execution");
    }
    if receipt.evidence_ref != row.evidence_ref || record.relative_path != row.evidence_ref {
        bail!(
            "receipt locator {} / {} does not match required {}",
            receipt.evidence_ref,
            record.relative_path,
            row.evidence_ref
        );
    }
    if receipt.surface != row.surface || receipt.mode != row.mode {
        bail!("receipt surface or mode does not match the matrix requirement");
    }
    let expected_target_os = expected_target_os(&row.surface);
    if receipt.target_os != expected_target_os {
        bail!(
            "receipt target_os {} does not match required {}",
            receipt.target_os,
            expected_target_os
        );
    }
    if receipt.os.is_empty() || receipt.arch.is_empty() {
        bail!("receipt host OS/arch is missing");
    }
    if row.surface == "desktop" && receipt.os != "windows" {
        bail!("desktop receipt must be captured on a Windows target host");
    }
    if row.surface == "docker" && receipt.os != "linux" {
        bail!("Docker receipt must be captured on a Linux target host");
    }
    if !receipt.command_fingerprint.starts_with("fnv1a64:")
        || receipt.command_fingerprint.len() != "fnv1a64:".len() + 16
    {
        bail!("receipt command fingerprint is malformed");
    }
    if row.surface == "android" {
        validate_android_claims(receipt, row)?;
    }
    if row.surface == "desktop"
        && matches!(
            row.evidence_id.as_str(),
            "smoke.desktop.local-backend" | "smoke.desktop.remote-browser"
        )
    {
        validate_desktop_claims(receipt, row)?;
    }
    validate_freshness(receipt, row, now)
}

fn validate_producer_binding(
    receipt: &Receipt,
    row: &MatrixRow,
    binding: &ReceiptProducerBinding,
) -> Result<()> {
    if receipt.schema != 3 {
        bail!("tag-ready requires receipt schema 3");
    }
    if receipt.evidence_id != row.evidence_id {
        bail!("receipt evidence ID does not match the matrix requirement");
    }
    if receipt.producer_id != binding.producer_id {
        bail!(
            "receipt producer {} does not match registry owner {}",
            receipt.producer_id,
            binding.producer_id
        );
    }
    if receipt.producer_contract != binding.contract_fingerprint {
        bail!("receipt producer contract does not match the current registry");
    }
    if !receipt.execution_id.starts_with("exec-fnv1a64-")
        || receipt.execution_id.len() != "exec-fnv1a64-".len() + 16
    {
        bail!("receipt execution ID is malformed");
    }
    let mut expected_evidence = binding.evidence_ids.clone();
    expected_evidence.sort();
    let mut observed_evidence = receipt.execution_evidence_ids.clone();
    observed_evidence.sort();
    if observed_evidence != expected_evidence {
        bail!("receipt execution evidence set does not match its producer contract");
    }
    let mut expected_artifacts = binding.artifacts.clone();
    expected_artifacts.sort();
    let mut observed_artifacts = receipt.command_artifacts.clone();
    observed_artifacts.sort();
    if observed_artifacts != expected_artifacts {
        bail!("receipt command artifacts do not match its producer contract");
    }
    let expected_inputs = binding.bound_env.iter().cloned().collect::<BTreeSet<_>>();
    let observed_inputs = receipt
        .producer_inputs
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if observed_inputs != expected_inputs {
        bail!("receipt producer inputs do not match its producer contract");
    }
    if receipt
        .producer_inputs
        .values()
        .any(|value| value.is_empty())
    {
        bail!("receipt producer input contains an empty value");
    }
    Ok(())
}

fn validate_docker_candidate_identity(
    rows: &[MatrixRow],
    receipts: &BTreeMap<String, ReceiptRecord>,
) -> Result<()> {
    const IMAGE_ENV: &str = "DEVE_RELEASE_CANDIDATE_IMAGE";
    const IMAGE_ID_ENV: &str = "DEVE_RELEASE_CANDIDATE_IMAGE_ID";
    let mut image_refs = BTreeSet::new();
    let mut image_ids = BTreeSet::new();
    for row in rows.iter().filter(|row| {
        row.gate == "tag-ready"
            && row.requirement == "required"
            && row.surface == "docker"
            && row.evidence_kind == "receipt"
    }) {
        let Some(record) = receipts.get(&row.evidence_id) else {
            continue;
        };
        let image_ref = record
            .receipt
            .producer_inputs
            .get(IMAGE_ENV)
            .with_context(|| format!("{} is missing {IMAGE_ENV}", row.evidence_id))?;
        let image_id = record
            .receipt
            .producer_inputs
            .get(IMAGE_ID_ENV)
            .with_context(|| format!("{} is missing {IMAGE_ID_ENV}", row.evidence_id))?;
        if image_ref.trim().is_empty() {
            bail!("{} has an empty candidate image reference", row.evidence_id);
        }
        if !valid_sha256_image_id(image_id) {
            bail!(
                "{} has malformed candidate image ID {image_id}",
                row.evidence_id
            );
        }
        image_refs.insert(image_ref.clone());
        image_ids.insert(image_id.clone());
    }
    if image_refs.len() > 1 || image_ids.len() > 1 {
        bail!("Docker receipts were not produced from one exact candidate image");
    }
    Ok(())
}

fn valid_sha256_image_id(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn validate_freshness(receipt: &Receipt, row: &MatrixRow, now: DateTime<Utc>) -> Result<()> {
    let started = DateTime::parse_from_rfc3339(&receipt.started_at)
        .context("receipt started_at is not RFC3339")?
        .with_timezone(&Utc);
    let finished = DateTime::parse_from_rfc3339(&receipt.finished_at)
        .context("receipt finished_at is not RFC3339")?
        .with_timezone(&Utc);
    if started > finished {
        bail!("receipt started_at is after finished_at");
    }
    if started > now + Duration::minutes(5) || finished > now + Duration::minutes(5) {
        bail!("receipt timestamp is in the future");
    }
    if row.freshness == "target-host-30d" && now - finished > Duration::days(30) {
        bail!("target-host receipt is older than 30 days");
    }
    Ok(())
}

fn expected_target_os(surface: &str) -> &str {
    match surface {
        "web" => "web",
        "docker" => "linux",
        "desktop" => "windows",
        "android" => "android",
        "release" => "multi-platform",
        "github" => "github",
        other => other,
    }
}

fn read_receipts(root: &Path) -> Result<BTreeMap<String, ReceiptRecord>> {
    if !root.is_dir() {
        bail!(
            "acceptance-matrix tag-ready: receipt directory is missing: {}",
            root.display()
        );
    }
    let reader = ReceiptArtifactRoot::open(root)?;
    let mut budget = ReceiptArtifactBudget::default();
    let mut receipts = BTreeMap::new();
    for (relative_path, path) in reader.json_files()? {
        let content = reader.read_json(&path, &mut budget)?;
        let receipt: Receipt = serde_json::from_slice(&content)
            .with_context(|| format!("invalid acceptance receipt {}", path.display()))?;
        let evidence_id = receipt.evidence_id.clone();
        if receipts
            .insert(
                evidence_id.clone(),
                ReceiptRecord {
                    relative_path,
                    receipt,
                },
            )
            .is_some()
        {
            bail!("duplicate receipt for evidence_id {evidence_id}");
        }
    }
    validate_execution_groups(
        receipts.values().map(|record| &record.receipt),
        "acceptance-matrix tag-ready",
    )?;
    Ok(receipts)
}

fn git_status(root: &Path) -> Result<String> {
    git_output(
        root,
        [
            "-c",
            "status.showUntrackedFiles=all",
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ],
    )
}

fn git_output<const N: usize>(root: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("acceptance-matrix tag-ready: failed to run git")?;
    if !output.status.success() {
        bail!("acceptance-matrix tag-ready: git command failed");
    }
    String::from_utf8(output.stdout)
        .context("acceptance-matrix tag-ready: git output was not UTF-8")
        .map(|value| value.trim().to_string())
}

#[cfg(test)]
mod tests;
