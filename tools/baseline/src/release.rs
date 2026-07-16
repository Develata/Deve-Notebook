//! plan_ref: infra

mod toolchain;
mod workflow;

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("release-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/release.tsv"))?;
    toolchain::check(ctx.root())?;
    workflow::check(ctx.root())?;
    ctx.ok();
    Ok(())
}
