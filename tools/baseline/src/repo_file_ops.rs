//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!   - 03_storage/projection#projection-contract

use crate::context::BaselineContext;
use crate::spec::{RunMode, run_tsv_with_mode};
use anyhow::Result;

pub fn run() -> Result<()> {
    run_with_mode(RunMode::Full)
}

fn run_with_mode(mode: RunMode) -> Result<()> {
    let ctx = BaselineContext::new("repo-file-ops-baseline")?;
    run_tsv_with_mode(&ctx, include_str!("specs/repo_file_ops.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
