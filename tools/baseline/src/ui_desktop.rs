//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!   - 11_ui_design/01_web#single-binary-distribution
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/index#context-action-surface
//!   - 11_ui_design/index#layout-navigation-and-focus

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
    let ctx = BaselineContext::new("ui-desktop-baseline-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/ui_desktop.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
