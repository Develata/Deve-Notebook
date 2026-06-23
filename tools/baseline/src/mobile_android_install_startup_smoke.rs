//! plan_ref: infra

use crate::context::BaselineContext;
use crate::mobile_shell_gate::{
    assert_android_shell_boundary, assert_positive_integer, parse_binary_flag,
};
use anyhow::Result;
use std::env;

const LABEL: &str = "mobile-android-install-startup-smoke-check";

pub fn run() -> Result<()> {
    let required = env::var("DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED")
        .unwrap_or_else(|_| "0".to_string());
    let uninstall_after =
        env::var("DEVE_MOBILE_ANDROID_INSTALL_SMOKE_UNINSTALL").unwrap_or_else(|_| "1".to_string());
    let adb_timeout =
        env::var("DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS").unwrap_or_else(|_| "60".to_string());
    let startup_wait =
        env::var("DEVE_MOBILE_ANDROID_STARTUP_WAIT_SECS").unwrap_or_else(|_| "3".to_string());

    let ctx = BaselineContext::new(LABEL)?;
    assert_android_shell_boundary(
        ctx.root(),
        LABEL,
        "iOS generated project is not allowed in the Android install/startup gate",
    )?;

    let required = parse_binary_flag(
        LABEL,
        "DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED",
        &required,
    )?;
    parse_binary_flag(
        LABEL,
        "DEVE_MOBILE_ANDROID_INSTALL_SMOKE_UNINSTALL",
        &uninstall_after,
    )?;

    if required {
        assert_positive_integer(LABEL, "DEVE_MOBILE_ANDROID_ADB_TIMEOUT_SECS", &adb_timeout)?;
        assert_positive_integer(
            LABEL,
            "DEVE_MOBILE_ANDROID_STARTUP_WAIT_SECS",
            &startup_wait,
        )?;
    }

    ctx.ok();
    Ok(())
}
