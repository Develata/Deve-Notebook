//! plan_ref:
//!   - 13_i18n#i18n-error-code-catalog
//!   - 07_network#server-ws-runtime

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("ws-structured-errors-check")?;
    run_tsv(&ctx, include_str!("specs/ws_structured_errors.tsv"))?;
    ctx.ok();
    Ok(())
}
