//! plan_ref:
//!   - 05_diff_logic#authority-diff-core
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 05_diff_logic#remote-projection-transport
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 12_source_control_ui#external-changes-sibling-view

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
    let ctx = BaselineContext::new("source-control-baseline-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/source_control.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
