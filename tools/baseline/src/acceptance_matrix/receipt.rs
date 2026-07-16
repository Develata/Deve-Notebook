//! Command execution and schema 3 receipt writer for release evidence.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

mod executor;
mod model;
mod process;
mod publication;

pub(super) use executor::execute_and_write;
pub(super) use model::{CommandStep, EvidenceSpec, ExecutionSpec, Receipt};
pub(in crate::acceptance_matrix) use process::run_step;
pub(super) use publication::ensure_output_outside_worktree;

const DEFAULT_TIMEOUT_SECS: u64 = 3_600;

pub(crate) fn run(args: &[String]) -> Result<()> {
    let parsed = ReceiptArgs::parse(args)?;
    let ctx = BaselineContext::new("acceptance-receipt")?;
    ensure_output_outside_worktree(ctx.root(), &parsed.evidence.output)?;
    execute_and_write(
        ctx.root(),
        &[parsed.evidence],
        &ExecutionSpec {
            producer_id: "manual.unbound".to_string(),
            producer_contract: "unbound".to_string(),
            command_artifacts: Vec::new(),
            producer_inputs: BTreeMap::new(),
            steps: vec![CommandStep {
                program: parsed.command[0].clone(),
                args: parsed.command[1..].to_vec(),
                env: BTreeMap::new(),
            }],
            finally_steps: Vec::new(),
            timeout: Duration::from_secs(parsed.timeout_secs),
        },
    )
}

struct ReceiptArgs {
    evidence: EvidenceSpec,
    timeout_secs: u64,
    command: Vec<String>,
}

impl ReceiptArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let separator = args
            .iter()
            .position(|arg| arg == "--")
            .context("acceptance-receipt: expected `--` before the command")?;
        let mut values = BTreeMap::new();
        let mut claims = None;
        let mut timeout_secs = DEFAULT_TIMEOUT_SECS;
        let mut index = 0usize;
        while index < separator {
            let option = &args[index];
            let value = args
                .get(index + 1)
                .with_context(|| format!("acceptance-receipt: missing value for {option}"))?;
            match option.as_str() {
                "--evidence-id" | "--evidence-ref" | "--surface" | "--mode" | "--target-os"
                | "--output" => {
                    if values.insert(option.clone(), value.clone()).is_some() {
                        bail!("acceptance-receipt: duplicate option {option}");
                    }
                }
                "--claims" => claims = Some(PathBuf::from(value)),
                "--timeout-secs" => {
                    timeout_secs = value
                        .parse::<u64>()
                        .context("acceptance-receipt: --timeout-secs must be an integer")?;
                    if timeout_secs == 0 {
                        bail!("acceptance-receipt: --timeout-secs must be positive");
                    }
                }
                other => bail!("acceptance-receipt: unknown option {other}"),
            }
            index += 2;
        }
        let command = args[separator + 1..].to_vec();
        if command.is_empty() {
            bail!("acceptance-receipt: command is required after `--`");
        }
        let required = |name: &str| {
            values
                .get(name)
                .cloned()
                .with_context(|| format!("acceptance-receipt: {name} is required"))
        };
        Ok(Self {
            evidence: EvidenceSpec {
                evidence_id: required("--evidence-id")?,
                evidence_ref: required("--evidence-ref")?,
                surface: required("--surface")?,
                mode: required("--mode")?,
                target_os: required("--target-os")?,
                output: PathBuf::from(required("--output")?),
                claims,
            },
            timeout_secs,
            command,
        })
    }
}

#[cfg(test)]
#[path = "receipt/tests.rs"]
mod tests;
