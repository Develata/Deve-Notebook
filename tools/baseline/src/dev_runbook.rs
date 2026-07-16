//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("check-dev-runbook-baseline")?;
    run_tsv(&ctx, include_str!("specs/dev_runbook.tsv"))?;
    ctx.ok();
    Ok(())
}
