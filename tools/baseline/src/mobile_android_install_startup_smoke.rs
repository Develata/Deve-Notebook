//! plan_ref: infra

use crate::context::BaselineContext;
use crate::env_gate::binary_flag_from_env;
use crate::mobile_shell_gate::{assert_android_shell_boundary, assert_positive_integer};
use anyhow::Result;
use std::env;

const LABEL: &str = "mobile-android-install-startup-smoke-check";

pub fn run() -> Result<()> {
    let required = binary_flag_from_env(
        LABEL,
        "DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED",
        false,
    )?;
    binary_flag_from_env(LABEL, "DEVE_MOBILE_ANDROID_INSTALL_SMOKE_UNINSTALL", true)?;
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
