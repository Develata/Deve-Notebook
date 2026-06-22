//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("check-source-control-smoke-hygiene")?;
    run_tsv(&ctx, include_str!("specs/source_control_smoke_hygiene.tsv"))?;
    ctx.ok();
    Ok(())
}
