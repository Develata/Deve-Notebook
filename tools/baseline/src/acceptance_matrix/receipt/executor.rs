//! Bounded process execution and atomic receipt publication.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::model::{EvidenceSpec, ExecutionSpec, Receipt};
use super::process::run_step;
use super::publication::{ensure_output_outside_worktree, write_batch_atomic};
use crate::acceptance_matrix::receipt_limits::{
    MAX_EXECUTION_RECEIPTS, add_total_bytes, read_json_bounded, validate_file_size,
};
use anyhow::{Context, Result, bail};
use chrono::{SecondsFormat, Utc};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(in crate::acceptance_matrix) fn execute_and_write(
    root: &Path,
    evidence: &[EvidenceSpec],
    execution_spec: &ExecutionSpec,
) -> Result<()> {
    if evidence.is_empty() {
        bail!("acceptance-receipt: at least one evidence binding is required");
    }
    if evidence.len() > MAX_EXECUTION_RECEIPTS {
        bail!(
            "acceptance-receipt: atomic evidence group exceeds {MAX_EXECUTION_RECEIPTS} receipts"
        );
    }
    if execution_spec.steps.is_empty() {
        bail!("acceptance-receipt: at least one command step is required");
    }
    if execution_spec.producer_id.trim().is_empty()
        || execution_spec.producer_contract.trim().is_empty()
    {
        bail!("acceptance-receipt: producer binding is required");
    }
    let mut evidence_ids = evidence
        .iter()
        .map(|item| item.evidence_id.clone())
        .collect::<Vec<_>>();
    evidence_ids.sort();
    evidence_ids.dedup();
    if evidence_ids.len() != evidence.len() {
        bail!("acceptance-receipt: duplicate evidence binding");
    }
    for item in evidence {
        ensure_output_outside_worktree(root, &item.output)?;
        if item.output.exists() {
            bail!(
                "acceptance-receipt: output already exists: {}",
                item.output.display()
            );
        }
        if !item.output.ends_with(Path::new(&item.evidence_ref)) {
            bail!(
                "acceptance-receipt: output {} must end with evidence locator {}",
                item.output.display(),
                item.evidence_ref
            );
        }
    }

    let head = git_output(root, ["rev-parse", "HEAD"])?;
    let dirty_before = !git_status(root)?.is_empty();
    let started_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let started = Instant::now();
    let mut step_status = None;
    let mut errors = Vec::new();

    for step in &execution_spec.steps {
        let remaining = execution_spec.timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            errors.push("producer timeout expired before the next command step".to_string());
            break;
        }
        match run_step(root, step, remaining) {
            Ok(status) => {
                step_status = Some(status);
                if !status.success() {
                    errors.push(format!(
                        "command step {} returned a non-zero exit status",
                        step.program
                    ));
                    break;
                }
            }
            Err(error) => {
                errors.push(error.to_string());
                break;
            }
        }
    }
    for step in &execution_spec.finally_steps {
        match run_step(root, step, Duration::from_secs(60)) {
            Ok(status) if status.success() => {}
            Ok(_) => errors.push(format!("cleanup step {} failed", step.program)),
            Err(error) => errors.push(format!("cleanup step {} failed: {error}", step.program)),
        }
    }

    let finished_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let head_after_result = git_output(root, ["rev-parse", "HEAD"]);
    let dirty_after_result = git_status(root);
    let head_after = head_after_result.as_ref().ok().cloned();
    let dirty_after = dirty_after_result
        .as_ref()
        .map_or(true, |value| !value.is_empty());
    collect_repository_errors(
        &head,
        head_after.as_deref(),
        dirty_before,
        dirty_after,
        &head_after_result,
        &dirty_after_result,
        &mut errors,
    );

    let command = command_descriptor(execution_spec);
    let execution_id = execution_id(execution_spec, &head, &started_at);
    let base_passed = errors.is_empty() && step_status.is_some_and(|status| status.success());
    let mut claims_total = 0u64;
    let mut receipts = Vec::new();
    for item in evidence {
        let mut item_errors = errors.clone();
        let claims = read_claims(item.claims.as_deref(), &mut item_errors, &mut claims_total);
        let passed = base_passed && item_errors.is_empty();
        let receipt = build_receipt(
            item,
            execution_spec,
            &command,
            &execution_id,
            &evidence_ids,
            &head,
            head_after.clone(),
            dirty_before,
            dirty_after,
            &started_at,
            &finished_at,
            step_status,
            claims,
            item_errors,
            passed,
        );
        receipts.push((item.output.clone(), item.evidence_id.clone(), receipt));
    }
    let publications = serialize_receipts_bounded(&mut receipts)?;
    let failed = receipts
        .iter()
        .filter(|(_, _, receipt)| receipt.status != "passed")
        .map(|(_, evidence_id, _)| evidence_id.clone())
        .collect::<Vec<_>>();
    write_batch_atomic(root, &publications, &execution_id)?;
    for item in evidence
        .iter()
        .filter(|item| !failed.contains(&item.evidence_id))
    {
        println!(
            "acceptance-receipt: passed {} -> {}",
            item.evidence_id,
            item.output.display()
        );
    }
    if failed.is_empty() {
        Ok(())
    } else {
        bail!(
            "acceptance-receipt: producer failed for evidence: {}",
            failed.join(", ")
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn build_receipt(
    item: &EvidenceSpec,
    execution_spec: &ExecutionSpec,
    command: &[String],
    execution_id: &str,
    execution_evidence_ids: &[String],
    head: &str,
    head_after: Option<String>,
    dirty_before: bool,
    dirty_after: bool,
    started_at: &str,
    finished_at: &str,
    step_status: Option<ExitStatus>,
    claims: Option<Value>,
    errors: Vec<String>,
    passed: bool,
) -> Receipt {
    Receipt {
        schema: 3,
        producer_id: execution_spec.producer_id.clone(),
        producer_contract: execution_spec.producer_contract.clone(),
        execution_id: execution_id.to_string(),
        execution_evidence_ids: execution_evidence_ids.to_vec(),
        evidence_id: item.evidence_id.clone(),
        evidence_ref: item.evidence_ref.clone(),
        head: head.to_string(),
        head_after,
        dirty_before,
        dirty_after,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        target_os: item.target_os.clone(),
        surface: item.surface.clone(),
        mode: item.mode.clone(),
        started_at: started_at.to_string(),
        finished_at: finished_at.to_string(),
        status: if passed { "passed" } else { "failed" }.to_string(),
        exit_code: step_status.and_then(|status| status.code()),
        error: (!errors.is_empty()).then(|| errors.join("; ")),
        command_program: execution_spec.steps[0].program.clone(),
        command_arg_count: execution_spec
            .steps
            .iter()
            .map(|step| step.args.len())
            .sum(),
        command_fingerprint: command_fingerprint(command),
        command_artifacts: execution_spec.command_artifacts.clone(),
        producer_inputs: execution_spec.producer_inputs.clone(),
        claims,
    }
}

fn collect_repository_errors(
    head: &str,
    head_after: Option<&str>,
    dirty_before: bool,
    dirty_after: bool,
    head_after_result: &Result<String>,
    dirty_after_result: &Result<String>,
    errors: &mut Vec<String>,
) {
    if dirty_before {
        errors.push("worktree was dirty before command execution".to_string());
    }
    if dirty_after {
        errors.push("worktree was dirty after command execution".to_string());
    }
    if let Err(error) = head_after_result {
        errors.push(format!("failed to read HEAD after command: {error}"));
    }
    if let Err(error) = dirty_after_result {
        errors.push(format!(
            "failed to read worktree state after command: {error}"
        ));
    }
    if head_after.is_some_and(|after| after != head) {
        errors.push("HEAD changed during command execution".to_string());
    }
}

pub(super) fn read_claims(
    path: Option<&Path>,
    errors: &mut Vec<String>,
    claims_total: &mut u64,
) -> Option<Value> {
    path.and_then(|path| {
        match read_json_bounded(path, "acceptance-receipt: claims JSON").and_then(|content| {
            let claims = serde_json::from_slice(&content).context("claims file is not JSON")?;
            add_total_bytes(
                "acceptance-receipt: claims group",
                claims_total,
                content.len() as u64,
            )?;
            Ok(claims)
        }) {
            Ok(claims) => Some(claims),
            Err(error) => {
                errors.push(error.to_string());
                None
            }
        }
    })
}

pub(super) fn serialize_receipts_bounded(
    receipts: &mut [(std::path::PathBuf, String, Receipt)],
) -> Result<Vec<(std::path::PathBuf, Vec<u8>)>> {
    if receipts
        .iter()
        .any(|(_, _, receipt)| receipt.status != "passed")
    {
        fail_receipt_group(
            receipts,
            "execution group failed because at least one evidence receipt failed",
            false,
        );
    }
    match serialize_receipt_group(receipts) {
        Ok(publications) => Ok(publications),
        Err(_) => {
            fail_receipt_group(
                receipts,
                "serialized receipt group exceeded the bounded publication limit; claims were omitted",
                true,
            );
            serialize_receipt_group(receipts)
                .context("acceptance-receipt: bounded failed receipt group could not be serialized")
        }
    }
}

fn serialize_receipt_group(
    receipts: &[(std::path::PathBuf, String, Receipt)],
) -> Result<Vec<(std::path::PathBuf, Vec<u8>)>> {
    let mut publications = Vec::with_capacity(receipts.len());
    let mut total = 0u64;
    for (output, _, receipt) in receipts {
        let content = serde_json::to_vec_pretty(receipt)?;
        validate_file_size(
            "acceptance-receipt: serialized receipt",
            content.len() as u64,
        )?;
        add_total_bytes(
            "acceptance-receipt: serialized receipt group",
            &mut total,
            content.len() as u64,
        )?;
        publications.push((output.clone(), content));
    }
    Ok(publications)
}

fn fail_receipt_group(
    receipts: &mut [(std::path::PathBuf, String, Receipt)],
    message: &str,
    omit_claims: bool,
) {
    for (_, _, receipt) in receipts {
        if omit_claims {
            receipt.claims = None;
        }
        mark_receipt_failed(receipt, message);
    }
}

fn mark_receipt_failed(receipt: &mut Receipt, message: &str) {
    receipt.status = "failed".into();
    receipt.error = Some(match receipt.error.take() {
        Some(existing) if !existing.is_empty() => format!("{existing}; {message}"),
        _ => message.to_string(),
    });
}

fn command_descriptor(spec: &ExecutionSpec) -> Vec<String> {
    let mut result = Vec::new();
    for (name, value) in &spec.producer_inputs {
        result.push(format!("<producer-input:{name}>"));
        result.push(value.clone());
    }
    for (index, step) in spec.steps.iter().enumerate() {
        if index > 0 {
            result.push("<next-step>".to_string());
        }
        result.push(step.program.clone());
        result.extend((0..step.args.len()).map(|_| "<arg>".to_string()));
    }
    result
}

fn execution_id(spec: &ExecutionSpec, head: &str, started_at: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let material = [
        spec.producer_id.clone(),
        spec.producer_contract.clone(),
        head.to_string(),
        started_at.to_string(),
        std::process::id().to_string(),
        nonce.to_string(),
    ];
    format!("exec-{}", command_fingerprint(&material).replace(':', "-"))
}

pub(super) fn command_fingerprint(command: &[String]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for argument in command {
        for byte in (argument.len() as u64)
            .to_le_bytes()
            .into_iter()
            .chain(argument.bytes())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    format!("fnv1a64:{hash:016x}")
}

fn git_output<const N: usize>(root: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("acceptance-receipt: failed to run git")?;
    if !output.status.success() {
        bail!("acceptance-receipt: git command failed");
    }
    String::from_utf8(output.stdout)
        .context("acceptance-receipt: git output was not UTF-8")
        .map(|value| value.trim().to_string())
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
