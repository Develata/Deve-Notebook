//! plan_ref:
//!   - 10_rendering#current-rendering-split
//!   - 10_rendering#markdown-render-whitelist
//!   - 10_rendering#link-activation-gate
//!   - 10_rendering#code-block-toolbar-contract
//!   - 10_rendering#outline-projection
//!   - 10_rendering#document-authority-bridge

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("rendering-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/rendering.tsv"))?;
    ctx.ok();
    Ok(())
}
