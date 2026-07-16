//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

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
    let ctx = BaselineContext::new("mobile-baseline-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/mobile.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
