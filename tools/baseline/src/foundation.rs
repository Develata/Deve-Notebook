//! plan_ref:
//!   - 01_terminology#normative-language
//!   - 01_terminology#core-definitions
//!   - 02_positioning#core-boundaries

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
    let ctx = BaselineContext::new("foundation-baseline-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/foundation.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
