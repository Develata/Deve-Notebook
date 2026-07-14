//! Fresh receipt validation for first-tag requirements.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use super::model::MatrixRow;
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Receipt {
    schema: u8,
    evidence_id: String,
    evidence_ref: String,
    head: String,
    head_after: Option<String>,
    dirty_before: bool,
    dirty_after: bool,
    os: String,
    arch: String,
    target_os: String,
    surface: String,
    mode: String,
    finished_at: String,
    status: String,
    command_fingerprint: String,
    command_program: String,
    #[serde(default)]
    command_artifacts: Vec<String>,
    claims: Option<Value>,
}

#[derive(Debug)]
struct ReceiptRecord {
    relative_path: String,
    receipt: Receipt,
}

pub(super) fn validate(root: &Path, rows: &[MatrixRow], receipt_dir: &Path) -> Result<()> {
    let head = git_output(root, ["rev-parse", "HEAD"])?;
    if !git_output(root, ["status", "--porcelain"])?.is_empty() {
        bail!("acceptance-matrix tag-ready: current worktree is dirty");
    }
    let receipts = read_receipts(receipt_dir)?;
    let mut blockers = Vec::new();
    for row in rows
        .iter()
        .filter(|row| row.gate == "tag-ready" && row.requirement == "required")
    {
        if row.evidence_kind == "gap" {
            blockers.push(format!(
                "{} remains an explicit gap: {}",
                row.requirement_id, row.note
            ));
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
        if let Err(error) = validate_receipt(record, row, &head, Utc::now()) {
            blockers.push(format!("{}: {error}", row.requirement_id));
        }
    }
    if blockers.is_empty() {
        println!("acceptance-matrix tag-ready: ok");
        Ok(())
    } else {
        bail!(
            "acceptance-matrix tag-ready blocked by {} requirement(s):\n- {}",
            blockers.len(),
            blockers.join("\n- ")
        )
    }
}

fn validate_receipt(
    record: &ReceiptRecord,
    row: &MatrixRow,
    head: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let receipt = &record.receipt;
    if !matches!(receipt.schema, 2 | 3) {
        bail!("unsupported receipt schema {}", receipt.schema);
    }
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
    if receipt.surface != row.surface {
        bail!(
            "receipt surface {} does not match required {}",
            receipt.surface,
            row.surface
        );
    }
    if receipt.mode != row.mode {
        bail!(
            "receipt mode {} does not match required {}",
            receipt.mode,
            row.mode
        );
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
    let finished = DateTime::parse_from_rfc3339(&receipt.finished_at)
        .context("receipt finished_at is not RFC3339")?
        .with_timezone(&Utc);
    if finished > now + Duration::minutes(5) {
        bail!("receipt finished_at is in the future");
    }
    if row.freshness == "target-host-30d" && now - finished > Duration::days(30) {
        bail!("target-host receipt is older than 30 days");
    }
    Ok(())
}

fn validate_android_claims(receipt: &Receipt, row: &MatrixRow) -> Result<()> {
    if receipt.schema != 3 {
        bail!("Android writable evidence requires receipt schema 3 typed claims");
    }
    let claims = receipt
        .claims
        .as_ref()
        .context("Android receipt is missing typed target/probe claims")?;
    let expected_producer = match row.mode.as_str() {
        "local-backend" => "smoke-mobile-android-lifecycle",
        "remote-browser" => "smoke-mobile-android-remote-browser",
        other => bail!("unsupported Android evidence mode {other}"),
    };
    let expected_artifact = match row.mode.as_str() {
        "local-backend" => "smoke-mobile-android-lifecycle.sh",
        "remote-browser" => "smoke-mobile-android-remote-browser.sh",
        _ => unreachable!(),
    };
    if claims.get("producer").and_then(Value::as_str) != Some(expected_producer) {
        bail!("Android claims producer does not match {expected_producer}");
    }
    if !receipt
        .command_artifacts
        .iter()
        .any(|artifact| artifact == expected_artifact)
    {
        bail!("Android receipt command is not bound to {expected_artifact}");
    }
    if claims.get("mode").and_then(Value::as_str) != Some(row.mode.as_str()) {
        bail!("Android claims mode does not match receipt mode");
    }
    let target = claims
        .get("target")
        .context("Android claims target is missing")?;
    let sdk = target.get("sdkLevel").and_then(Value::as_u64).unwrap_or(0);
    let provider_major = target
        .get("webViewProviderMajor")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if sdk < 29
        || provider_major < 137
        || target.get("supportBaseline").and_then(Value::as_bool) != Some(true)
    {
        bail!("Android target does not meet API 29 / WebView 137 support baseline");
    }
    let webcrypto = claims
        .get("webcrypto")
        .context("Android claims WebCrypto probe is missing")?;
    if webcrypto.get("writable").and_then(Value::as_bool) != Some(true)
        || !webcrypto.get("blocker").is_some_and(Value::is_null)
    {
        bail!("Android target did not pass the real Ed25519 writer probe");
    }
    let journey = claims
        .get("journey")
        .context("Android claims journey result is missing")?;
    if journey
        .get("writableLifecycleComplete")
        .and_then(Value::as_bool)
        != Some(true)
    {
        bail!("Android writable lifecycle journey is incomplete");
    }
    let executable = Path::new(&receipt.command_program)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(executable.as_str(), "bash" | "bash.exe" | "sh" | "sh.exe") {
        bail!("Android receipt producer must be invoked through the shell harness");
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
    let mut files = Vec::new();
    collect_json(root, &mut files)?;
    files.sort();
    let mut receipts = BTreeMap::new();
    for path in files {
        let content = fs::read_to_string(&path)?;
        let receipt: Receipt = serde_json::from_str(&content)
            .with_context(|| format!("invalid acceptance receipt {}", path.display()))?;
        let evidence_id = receipt.evidence_id.clone();
        let relative_path = path
            .strip_prefix(root)
            .context("acceptance receipt escaped receipt root")?
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
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
    Ok(receipts)
}

fn collect_json(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_json(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(())
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
mod tests {
    use super::{Receipt, ReceiptRecord, validate_receipt};
    use crate::acceptance_matrix::model::MatrixRow;
    use chrono::{Duration, Utc};
    use serde_json::json;

    #[test]
    fn target_host_receipt_rejects_dirty_and_stale_evidence() {
        let now = Utc::now();
        let row = MatrixRow {
            requirement_id: "journey.android".into(),
            journey_id: "android-local-backend".into(),
            flow_id: "none".into(),
            case_id: "none".into(),
            surface: "android".into(),
            mode: "local-backend".into(),
            gate: "tag-ready".into(),
            requirement: "required".into(),
            evidence_kind: "receipt".into(),
            evidence_id: "smoke.android".into(),
            evidence_ref: "receipts/smoke.android.json".into(),
            freshness: "target-host-30d".into(),
            note: String::new(),
        };
        let mut record = ReceiptRecord {
            relative_path: "receipts/smoke.android.json".into(),
            receipt: Receipt {
                schema: 3,
                evidence_id: "smoke.android".into(),
                evidence_ref: "receipts/smoke.android.json".into(),
                head: "abc".into(),
                head_after: Some("abc".into()),
                dirty_before: true,
                dirty_after: false,
                os: "linux".into(),
                arch: "x86_64".into(),
                target_os: "android".into(),
                surface: "android".into(),
                mode: "local-backend".into(),
                finished_at: now.to_rfc3339(),
                status: "passed".into(),
                command_fingerprint: "fnv1a64:0123456789abcdef".into(),
                command_program: "bash".into(),
                command_artifacts: vec!["smoke-mobile-android-lifecycle.sh".into()],
                claims: Some(json!({
                    "schema": 1,
                    "producer": "smoke-mobile-android-lifecycle",
                    "mode": "local-backend",
                    "target": {
                        "sdkLevel": 29,
                        "webViewProviderMajor": 137,
                        "supportBaseline": true
                    },
                    "webcrypto": { "writable": true, "blocker": null },
                    "journey": { "writableLifecycleComplete": true }
                })),
            },
        };
        assert!(validate_receipt(&record, &row, "abc", now).is_err());
        record.receipt.dirty_before = false;
        record.receipt.finished_at = (now - Duration::days(31)).to_rfc3339();
        assert!(validate_receipt(&record, &row, "abc", now).is_err());
        record.receipt.finished_at = now.to_rfc3339();
        assert!(validate_receipt(&record, &row, "abc", now).is_ok());
        record.receipt.command_artifacts.clear();
        assert!(validate_receipt(&record, &row, "abc", now).is_err());
        record.receipt.command_artifacts = vec!["smoke-mobile-android-lifecycle.sh".into()];
        record.receipt.claims.as_mut().unwrap()["webcrypto"]["writable"] = json!(false);
        record.receipt.claims.as_mut().unwrap()["webcrypto"]["blocker"] =
            json!("ed25519_unavailable");
        assert!(validate_receipt(&record, &row, "abc", now).is_err());
        record.receipt.claims.as_mut().unwrap()["webcrypto"]["writable"] = json!(true);
        record.receipt.claims.as_mut().unwrap()["webcrypto"]["blocker"] = json!(null);
        record.receipt.mode = "remote-browser".into();
        assert!(validate_receipt(&record, &row, "abc", now).is_err());
        record.receipt.mode = "local-backend".into();
        record.receipt.target_os = "linux".into();
        assert!(validate_receipt(&record, &row, "abc", now).is_err());
        record.receipt.target_os = "android".into();
        record.relative_path = "receipts/other.json".into();
        assert!(validate_receipt(&record, &row, "abc", now).is_err());
    }
}
