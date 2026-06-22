//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("ws-structured-errors-check")?;
    run_tsv(&ctx, include_str!("specs/ws_structured_errors.tsv"))?;
    ctx.ok();
    Ok(())
}
