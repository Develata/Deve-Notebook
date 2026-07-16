//! plan_ref:
//!   - 08_auth#unauthorized-disconnected-ui

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
    let ctx = BaselineContext::new("ui-disconnect-baseline-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/ui_disconnect.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
