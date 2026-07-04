//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::{RunMode, run_tsv_with_mode};
use anyhow::Result;

pub fn run() -> Result<()> {
    run_with_mode(RunMode::Full)
}

pub fn run_text() -> Result<()> {
    run_with_mode(RunMode::TextOnly)
}

fn run_with_mode(mode: RunMode) -> Result<()> {
    let ctx = BaselineContext::new("web-runtime-boundary-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/web_runtime_boundary.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
