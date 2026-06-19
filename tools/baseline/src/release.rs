//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("release-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/release.tsv"))?;
    ctx.ok();
    Ok(())
}
