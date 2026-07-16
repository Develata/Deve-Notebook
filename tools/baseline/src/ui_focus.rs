//! plan_ref:
//!   - 11_ui_design/index#layout-navigation-and-focus

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("ui-focus-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/ui_focus.tsv"))?;
    ctx.ok();
    Ok(())
}
