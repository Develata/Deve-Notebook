//! Machine-readable acceptance matrix and first-tag evidence gate.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use crate::context::BaselineContext;
use anyhow::{Result, bail};
use std::path::Path;

mod execution_group;
mod impact;
mod model;
mod parse;
mod producer;
mod receipt;
mod receipt_limits;
mod render;
mod tag_ready;
mod test_selector;
mod validate;

pub(crate) fn run(args: &[String]) -> Result<()> {
    let ctx = BaselineContext::new("acceptance-matrix")?;
    let rows = parse::read_matrix(ctx.root())?;
    validate::validate(ctx.root(), &rows)?;
    let producer_catalog = producer::validate_registry(ctx.root(), &rows)?;
    impact::validate(
        ctx.root(),
        &producer_catalog.producer_ids,
        &producer_catalog.ci_job_by_producer,
    )?;
    match args {
        [] => render::check_drift(ctx.root(), &rows)?,
        [action] if action == "--render" => render::write(ctx.root(), &rows)?,
        [action, receipt_dir] if action == "--tag-ready" => {
            let path = Path::new(receipt_dir);
            let path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                ctx.root().join(path)
            };
            tag_ready::validate(ctx.root(), &rows, &path)?;
        }
        _ => {
            bail!("acceptance-matrix: expected no args, `--render`, or `--tag-ready <receipt-dir>`")
        }
    }
    println!("acceptance-matrix: {} requirement(s) ok", rows.len());
    Ok(())
}

pub(crate) fn run_receipt(args: &[String]) -> Result<()> {
    receipt::run(args)
}

pub(crate) fn run_producers(args: &[String]) -> Result<()> {
    producer::run(args)
}

pub(crate) fn run_impact(args: &[String]) -> Result<()> {
    let ctx = BaselineContext::new("acceptance-impact")?;
    let rows = parse::read_matrix(ctx.root())?;
    validate::validate(ctx.root(), &rows)?;
    let producer_catalog = producer::validate_registry(ctx.root(), &rows)?;
    impact::run(ctx.root(), args, &producer_catalog, &rows)
}

pub(crate) fn run_impact_backlog() -> Result<()> {
    let ctx = BaselineContext::new("acceptance-impact-backlog")?;
    let rows = parse::read_matrix(ctx.root())?;
    validate::validate(ctx.root(), &rows)?;
    let producer_catalog = producer::validate_registry(ctx.root(), &rows)?;
    impact::validate(
        ctx.root(),
        &producer_catalog.producer_ids,
        &producer_catalog.ci_job_by_producer,
    )?;
    impact::run_backlog(&rows, &producer_catalog.executable_evidence_ids)
}

pub(crate) fn run_impact_shadow(args: &[String]) -> Result<()> {
    let ctx = BaselineContext::new("acceptance-impact-shadow")?;
    let rows = parse::read_matrix(ctx.root())?;
    validate::validate(ctx.root(), &rows)?;
    let producer_catalog = producer::validate_registry(ctx.root(), &rows)?;
    impact::run_shadow(ctx.root(), args, &producer_catalog, &rows)
}

pub(crate) fn collect_receipts(args: &[String]) -> Result<()> {
    producer::collect::run(args)
}
