//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-ios-shell-package-execution-gate

use crate::context::BaselineContext;
use crate::env_gate::binary_flag_from_env;
use crate::mobile_shell_gate::assert_ios_shell_boundary;
use anyhow::Result;

const LABEL: &str = "mobile-ios-install-startup-smoke-check";

pub fn run() -> Result<()> {
    binary_flag_from_env(
        LABEL,
        "DEVE_MOBILE_IOS_INSTALL_STARTUP_SMOKE_REQUIRED",
        false,
    )?;
    binary_flag_from_env(LABEL, "DEVE_MOBILE_IOS_BOOT_SIMULATOR", false)?;
    binary_flag_from_env(LABEL, "DEVE_MOBILE_IOS_INSTALL_SMOKE_TERMINATE", true)?;

    let ctx = BaselineContext::new(LABEL)?;
    assert_ios_shell_boundary(ctx.root(), LABEL)?;
    ctx.ok();
    Ok(())
}
