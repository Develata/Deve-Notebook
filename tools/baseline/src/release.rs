//! plan_ref:
//!   - 18_release#developer-baseline-checkers
//!   - 17_tech_stack#canonical-rust-toolchain
//!   - 18_release#first-tag-acceptance-matrix
//!   - 18_release#artifact-identity-and-integrity
//!   - 18_release#release-versioning

pub(crate) mod toolchain;
mod workflow;

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("release-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/release.tsv"))?;
    toolchain::check(ctx.root())?;
    workflow::check(ctx.root())?;
    ctx.ok();
    Ok(())
}
