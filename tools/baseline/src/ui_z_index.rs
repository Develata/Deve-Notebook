//! plan_ref:
//!   - 11_ui_design/index#layout-tokens-and-layer-registry

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("ui-z-index-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/ui_z_index.tsv"))?;
    ctx.ok();
    Ok(())
}
