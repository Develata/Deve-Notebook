//! plan_ref:
//!   - 11_ui_design/01_web#single-binary-distribution

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
    let ctx = BaselineContext::new("ui-spa-routing-baseline-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/ui_spa_routing.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
