//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("diff-color-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/diff_color.tsv"))?;
    ctx.ok();
    Ok(())
}
