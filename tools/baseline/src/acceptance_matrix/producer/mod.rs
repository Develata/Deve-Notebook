//! Rust-first acceptance producer planning, execution, and receipt aggregation.
//! plan_ref: 18_release#first-tag-acceptance-matrix

pub(super) mod artifact_reader;
pub(super) mod collect;
mod file_identity;
mod model;
mod plan;
mod registry;
mod runner;

use super::model::MatrixRow;
use super::receipt::ensure_output_outside_worktree;
use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use plan::{RunArgs, build_plan, preflight_execution, print_plan};
use runner::{run_producer, staging_directory};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(in crate::acceptance_matrix) struct ReceiptProducerBinding {
    pub(in crate::acceptance_matrix) producer_id: String,
    pub(in crate::acceptance_matrix) contract_fingerprint: String,
    pub(in crate::acceptance_matrix) evidence_ids: Vec<String>,
    pub(in crate::acceptance_matrix) artifacts: Vec<String>,
    pub(in crate::acceptance_matrix) bound_env: Vec<String>,
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let parsed = RunArgs::parse(args)?;
    let ctx = BaselineContext::new("acceptance-run")?;
    let rows = super::parse::read_matrix(ctx.root())?;
    super::validate::validate(ctx.root(), &rows)?;
    let registry = registry::read_and_validate(ctx.root(), &rows)?;
    let plans = build_plan(&parsed, &registry, &rows)?;
    print_plan(&parsed, &plans);
    if parsed.plan {
        return Ok(());
    }
    let receipt_dir = parsed
        .receipt_dir
        .as_ref()
        .context("acceptance-run: --receipt-dir is required unless --plan is used")?;
    let receipt_dir = absolute(ctx.root(), receipt_dir);
    ensure_output_outside_worktree(ctx.root(), &receipt_dir)?;
    preflight_execution(ctx.root(), &parsed, &plans)?;
    if plans.iter().all(|plan| !plan.selected) {
        println!(
            "acceptance-run: tier={} has no command producers",
            parsed.tier
        );
        return Ok(());
    }
    if receipt_dir.exists() {
        bail!(
            "acceptance-run: receipt directory already exists: {}",
            receipt_dir.display()
        );
    }
    let staging = staging_directory(&receipt_dir)?;
    ensure_output_outside_worktree(ctx.root(), &staging)?;
    fs::create_dir(&staging).with_context(|| {
        format!(
            "acceptance-run: failed to create staging directory {}",
            staging.display()
        )
    })?;
    let mut execution_error = None;
    for plan in plans.iter().filter(|plan| plan.selected) {
        if let Err(error) = run_producer(ctx.root(), &staging, plan) {
            execution_error = Some(error);
            break;
        }
    }
    if let Err(error) = ensure_output_outside_worktree(ctx.root(), &receipt_dir) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if receipt_dir.exists() {
        let _ = fs::remove_dir_all(&staging);
        bail!(
            "acceptance-run: receipt directory appeared during execution: {}",
            receipt_dir.display()
        );
    }
    if let Err(error) = fs::rename(&staging, &receipt_dir) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error).with_context(|| {
            format!(
                "acceptance-run: failed to publish receipt directory {}",
                receipt_dir.display()
            )
        });
    }
    if let Some(error) = execution_error {
        return Err(error).context(format!(
            "acceptance-run: failed receipts published at {}",
            receipt_dir.display()
        ));
    }
    println!(
        "acceptance-run: tier={} producer(s)={} ok",
        parsed.tier,
        plans.iter().filter(|plan| plan.selected).count()
    );
    Ok(())
}

pub(super) fn validate_registry(root: &Path, rows: &[MatrixRow]) -> Result<()> {
    registry::read_and_validate(root, rows).map(|_| ())
}

pub(in crate::acceptance_matrix) fn receipt_bindings(
    root: &Path,
    rows: &[MatrixRow],
) -> Result<BTreeMap<String, ReceiptProducerBinding>> {
    let registry = registry::read_and_validate(root, rows)?;
    let mut bindings = BTreeMap::new();
    for producer in &registry.producers {
        let binding = ReceiptProducerBinding {
            producer_id: producer.producer_id.clone(),
            contract_fingerprint: registry::contract_fingerprint(producer)?,
            evidence_ids: producer.evidence_ids.clone(),
            artifacts: producer.artifacts.clone(),
            bound_env: producer.bound_env.clone(),
        };
        for evidence_id in &producer.evidence_ids {
            bindings.insert(evidence_id.clone(), binding.clone());
        }
    }
    Ok(bindings)
}

fn absolute(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
