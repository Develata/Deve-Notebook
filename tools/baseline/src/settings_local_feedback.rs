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
    let ctx = BaselineContext::new("settings-local-feedback-baseline-check")?;
    run_tsv_with_mode(
        &ctx,
        include_str!("specs/settings_local_feedback.tsv"),
        mode,
    )?;
    ctx.ok();
    Ok(())
}
