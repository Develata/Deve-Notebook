//! plan_ref:
//!   - 17_tech_stack#graph-visualization

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("graph-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/graph.tsv"))?;
    ctx.ok();
    Ok(())
}
