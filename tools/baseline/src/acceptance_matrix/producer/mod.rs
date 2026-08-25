//! Rust-first acceptance producer planning, execution, and receipt aggregation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

pub(super) mod artifact_reader;
pub(super) mod collect;
mod execution_policy;
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
use runner::{run_producer, run_static_producer, staging_directory};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
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

pub(super) struct ProducerCatalog {
    pub(super) registry_fingerprint: String,
    pub(super) producer_ids: BTreeSet<String>,
    pub(super) evidence_by_producer: BTreeMap<String, Vec<String>>,
    pub(super) ci_job_by_producer: BTreeMap<String, String>,
    pub(super) executable_evidence_ids: BTreeSet<String>,
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
    preflight_execution(ctx.root(), &parsed, &plans)?;
    let selected = plans
        .iter()
        .filter(|plan| plan.selected)
        .collect::<Vec<_>>();
    let emits_receipts = selected.iter().any(|plan| plan.emits_receipts());
    let emits_static = selected.iter().any(|plan| !plan.emits_receipts());
    if emits_receipts && emits_static {
        bail!("acceptance-run: one tier may not mix receipt and test/script producers");
    }
    if emits_static {
        if parsed.receipt_dir.is_some() {
            bail!("acceptance-run: test/script producers do not accept --receipt-dir");
        }
        let state_root = staging_directory(&std::env::temp_dir().join("deve-acceptance-ci"))?;
        ensure_output_outside_worktree(ctx.root(), &state_root)?;
        fs::create_dir(&state_root)?;
        let result = selected
            .into_iter()
            .try_for_each(|plan| run_static_producer(ctx.root(), &state_root, plan));
        let cleanup = fs::remove_dir_all(&state_root);
        result?;
        cleanup.with_context(|| {
            format!(
                "acceptance-run: failed to remove static producer state {}",
                state_root.display()
            )
        })?;
        println!(
            "acceptance-run: tier={} producer(s)={} ok",
            parsed.tier,
            plans.iter().filter(|plan| plan.selected).count()
        );
        return Ok(());
    }
    let receipt_dir = parsed
        .receipt_dir
        .as_ref()
        .context("acceptance-run: --receipt-dir is required for receipt producers")?;
    let receipt_dir = absolute(ctx.root(), receipt_dir);
    ensure_output_outside_worktree(ctx.root(), &receipt_dir)?;
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

pub(super) fn validate_registry(root: &Path, rows: &[MatrixRow]) -> Result<ProducerCatalog> {
    let content = fs::read(root.join(model::PRODUCER_REGISTRY_PATH)).with_context(|| {
        format!(
            "acceptance producers: failed to fingerprint {}",
            model::PRODUCER_REGISTRY_PATH
        )
    })?;
    let registry = registry::read_and_validate(root, rows)?;
    let ci_job_by_producer = registry::validated_ci_job_map(root, &registry)?;
    let matrix_evidence = registry::matrix_executable_evidence(rows)?;
    let executable_evidence_ids = registry::executable_evidence_ids(&registry, &matrix_evidence)?;
    Ok(ProducerCatalog {
        registry_fingerprint: format!("sha256:{:x}", Sha256::digest(&content)),
        producer_ids: registry
            .producers
            .iter()
            .map(|producer| producer.producer_id.clone())
            .collect(),
        evidence_by_producer: registry
            .producers
            .into_iter()
            .map(|producer| (producer.producer_id, producer.evidence_ids))
            .collect(),
        ci_job_by_producer,
        executable_evidence_ids,
    })
}

pub(in crate::acceptance_matrix) fn receipt_bindings(
    root: &Path,
    rows: &[MatrixRow],
) -> Result<BTreeMap<String, ReceiptProducerBinding>> {
    let registry = registry::read_and_validate(root, rows)?;
    let receipt_ids = rows
        .iter()
        .filter(|row| row.evidence_kind == "receipt")
        .map(|row| row.evidence_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut bindings = BTreeMap::new();
    for producer in &registry.producers {
        let evidence_ids = producer
            .evidence_ids
            .iter()
            .filter(|id| receipt_ids.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if evidence_ids.is_empty() {
            continue;
        }
        let binding = ReceiptProducerBinding {
            producer_id: producer.producer_id.clone(),
            contract_fingerprint: registry::contract_fingerprint(producer)?,
            evidence_ids,
            artifacts: producer.artifacts.clone(),
            bound_env: producer.bound_env.clone(),
        };
        for evidence_id in &binding.evidence_ids {
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
