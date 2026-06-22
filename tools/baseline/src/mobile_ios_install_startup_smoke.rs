//! plan_ref: infra

use crate::context::BaselineContext;
use crate::mobile_shell_gate::assert_ios_shell_boundary;
use anyhow::Result;

const LABEL: &str = "mobile-ios-install-startup-smoke-check";

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    assert_ios_shell_boundary(ctx.root(), LABEL)?;
    ctx.ok();
    Ok(())
}
