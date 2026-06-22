//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("browser-prefs-boundary-check")?;
    run_tsv(&ctx, include_str!("specs/browser_prefs_boundary.tsv"))?;
    ctx.ok();
    Ok(())
}
