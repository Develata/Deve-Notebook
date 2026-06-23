//! plan_ref: infra

use crate::context::BaselineContext;
use crate::mobile_shell_gate::{assert_ios_shell_boundary, parse_binary_flag};
use anyhow::Result;
use std::env;

const LABEL: &str = "mobile-ios-install-startup-smoke-check";

pub fn run() -> Result<()> {
    let required =
        env::var("DEVE_MOBILE_IOS_INSTALL_STARTUP_SMOKE_REQUIRED").unwrap_or_else(|_| "0".into());
    let boot_simulator = env::var("DEVE_MOBILE_IOS_BOOT_SIMULATOR").unwrap_or_else(|_| "0".into());
    let terminate_after =
        env::var("DEVE_MOBILE_IOS_INSTALL_SMOKE_TERMINATE").unwrap_or_else(|_| "1".into());

    let ctx = BaselineContext::new(LABEL)?;
    assert_ios_shell_boundary(ctx.root(), LABEL)?;
    parse_binary_flag(
        LABEL,
        "DEVE_MOBILE_IOS_INSTALL_STARTUP_SMOKE_REQUIRED",
        &required,
    )?;
    parse_binary_flag(LABEL, "DEVE_MOBILE_IOS_BOOT_SIMULATOR", &boot_simulator)?;
    parse_binary_flag(
        LABEL,
        "DEVE_MOBILE_IOS_INSTALL_SMOKE_TERMINATE",
        &terminate_after,
    )?;
    ctx.ok();
    Ok(())
}
