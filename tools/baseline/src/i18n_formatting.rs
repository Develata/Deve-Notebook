//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("i18n-formatting-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/i18n_formatting.tsv"))?;
    ctx.ok();
    Ok(())
}
