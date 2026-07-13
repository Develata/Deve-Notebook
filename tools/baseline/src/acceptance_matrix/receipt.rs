//! Command receipt writer for release evidence.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize)]
struct Receipt {
    schema: u8,
    evidence_id: String,
    evidence_ref: String,
    head: String,
    head_after: Option<String>,
    dirty_before: bool,
    dirty_after: bool,
    os: &'static str,
    arch: &'static str,
    target_os: String,
    surface: String,
    mode: String,
    started_at: String,
    finished_at: String,
    status: &'static str,
    exit_code: Option<i32>,
    error: Option<String>,
    command_program: String,
    command_arg_count: usize,
    command_fingerprint: String,
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let parsed = ReceiptArgs::parse(args)?;
    let ctx = BaselineContext::new("acceptance-receipt")?;
    let output_absolute = if parsed.output.is_absolute() {
        parsed.output.clone()
    } else {
        ctx.root().join(&parsed.output)
    };
    if output_absolute.starts_with(ctx.root()) {
        bail!("acceptance-receipt: output must be outside the Git worktree");
    }
    if !parsed.output.ends_with(Path::new(&parsed.evidence_ref)) {
        bail!(
            "acceptance-receipt: output {} must end with evidence locator {}",
            parsed.output.display(),
            parsed.evidence_ref
        );
    }
    let head = git_output(ctx.root(), ["rev-parse", "HEAD"])?;
    let dirty_before = !git_output(ctx.root(), ["status", "--porcelain"])?.is_empty();
    let started_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let execution = Command::new(&parsed.command[0])
        .args(&parsed.command[1..])
        .current_dir(ctx.root())
        .status();
    let finished_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let head_after_result = git_output(ctx.root(), ["rev-parse", "HEAD"]);
    let dirty_after_result = git_output(ctx.root(), ["status", "--porcelain"]);
    let head_after = head_after_result.as_ref().ok().cloned();
    let dirty_after = dirty_after_result
        .as_ref()
        .map_or(true, |value| !value.is_empty());
    let mut errors = Vec::new();
    if let Err(error) = &execution {
        errors.push(error.to_string());
    }
    if execution.as_ref().is_ok_and(|status| !status.success()) {
        errors.push("command returned a non-zero exit status".to_string());
    }
    if dirty_before {
        errors.push("worktree was dirty before command execution".to_string());
    }
    if dirty_after {
        errors.push("worktree was dirty after command execution".to_string());
    }
    if let Err(error) = &head_after_result {
        errors.push(format!("failed to read HEAD after command: {error}"));
    }
    if let Err(error) = &dirty_after_result {
        errors.push(format!(
            "failed to read worktree state after command: {error}"
        ));
    }
    if head_after.as_deref().is_some_and(|after| after != head) {
        errors.push("HEAD changed during command execution".to_string());
    }
    let passed = execution.as_ref().is_ok_and(|status| status.success()) && errors.is_empty();
    let receipt = Receipt {
        schema: 2,
        evidence_id: parsed.evidence_id.clone(),
        evidence_ref: parsed.evidence_ref.clone(),
        head: head.clone(),
        head_after,
        dirty_before,
        dirty_after,
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        target_os: parsed.target_os.clone(),
        surface: parsed.surface.clone(),
        mode: parsed.mode.clone(),
        started_at,
        finished_at,
        status: if passed { "passed" } else { "failed" },
        exit_code: execution.as_ref().ok().and_then(|status| status.code()),
        error: (!errors.is_empty()).then(|| errors.join("; ")),
        command_program: parsed.command[0].clone(),
        command_arg_count: parsed.command.len().saturating_sub(1),
        command_fingerprint: command_fingerprint(&parsed.command),
    };
    write_atomic(&output_absolute, &serde_json::to_vec_pretty(&receipt)?)?;
    if passed {
        println!(
            "acceptance-receipt: passed {} -> {}",
            parsed.evidence_id,
            output_absolute.display()
        );
        Ok(())
    } else {
        bail!(
            "acceptance-receipt: command failed for {} with exit={:?} error={}",
            parsed.evidence_id,
            execution.as_ref().ok().and_then(|status| status.code()),
            receipt.error.as_deref().unwrap_or("none")
        )
    }
}

struct ReceiptArgs {
    evidence_id: String,
    evidence_ref: String,
    surface: String,
    mode: String,
    target_os: String,
    output: PathBuf,
    command: Vec<String>,
}

impl ReceiptArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let separator = args
            .iter()
            .position(|arg| arg == "--")
            .context("acceptance-receipt: expected `--` before the command")?;
        let mut evidence_id = None;
        let mut evidence_ref = None;
        let mut surface = None;
        let mut mode = None;
        let mut target_os = None;
        let mut output = None;
        let mut index = 0usize;
        while index < separator {
            let value = args.get(index + 1).with_context(|| {
                format!("acceptance-receipt: missing value for {}", args[index])
            })?;
            match args[index].as_str() {
                "--evidence-id" => evidence_id = Some(value.clone()),
                "--evidence-ref" => evidence_ref = Some(value.clone()),
                "--surface" => surface = Some(value.clone()),
                "--mode" => mode = Some(value.clone()),
                "--target-os" => target_os = Some(value.clone()),
                "--output" => output = Some(PathBuf::from(value)),
                other => bail!("acceptance-receipt: unsupported option {other}"),
            }
            index += 2;
        }
        let command = args[separator + 1..].to_vec();
        if command.is_empty() {
            bail!("acceptance-receipt: command is empty");
        }
        Ok(Self {
            evidence_id: evidence_id.context("acceptance-receipt: --evidence-id is required")?,
            evidence_ref: evidence_ref.context("acceptance-receipt: --evidence-ref is required")?,
            surface: surface.context("acceptance-receipt: --surface is required")?,
            mode: mode.context("acceptance-receipt: --mode is required")?,
            target_os: target_os.context("acceptance-receipt: --target-os is required")?,
            output: output.context("acceptance-receipt: --output is required")?,
            command,
        })
    }
}

fn command_fingerprint(command: &[String]) -> String {
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

fn write_atomic(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("receipt.json");
    let temp = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    fs::write(&temp, content)
        .with_context(|| format!("acceptance-receipt: failed to write {}", temp.display()))?;
    fs::rename(&temp, path)
        .with_context(|| format!("acceptance-receipt: failed to publish {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{ReceiptArgs, command_fingerprint, run};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn receipt_args_require_explicit_surface_and_command_boundary() {
        let args = [
            "--evidence-id",
            "smoke.web",
            "--evidence-ref",
            "receipts/smoke.web.json",
            "--surface",
            "web",
            "--mode",
            "browser",
            "--target-os",
            "web",
            "--output",
            "receipt.json",
            "--",
            "cargo",
            "test",
        ]
        .map(str::to_string);
        let parsed = ReceiptArgs::parse(&args).unwrap();
        assert_eq!(parsed.surface, "web");
        assert_eq!(parsed.mode, "browser");
        assert_eq!(parsed.command, ["cargo", "test"]);
        assert_eq!(
            command_fingerprint(&parsed.command),
            command_fingerprint(&parsed.command)
        );
    }

    #[test]
    fn failed_to_start_command_still_writes_failed_receipt() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = std::env::temp_dir().join(format!("deve-acceptance-receipt-{unique}.json"));
        let file_name = output.file_name().unwrap().to_string_lossy().to_string();
        let args = vec![
            "--evidence-id".to_string(),
            "test.failed-command".to_string(),
            "--evidence-ref".to_string(),
            file_name,
            "--surface".to_string(),
            "test".to_string(),
            "--mode".to_string(),
            "unit".to_string(),
            "--target-os".to_string(),
            std::env::consts::OS.to_string(),
            "--output".to_string(),
            output.display().to_string(),
            "--".to_string(),
            "deve-command-that-must-not-exist".to_string(),
        ];

        assert!(run(&args).is_err());
        let receipt: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(receipt["status"], "failed");
        assert!(receipt["exit_code"].is_null());
        let _ = fs::remove_file(output);
    }
}
