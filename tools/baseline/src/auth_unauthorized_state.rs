//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("auth-unauthorized-check")?;
    run_tsv(&ctx, include_str!("specs/auth_unauthorized_state.tsv"))?;
    ctx.ok();
    Ok(())
}
