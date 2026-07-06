//! plan_ref: infra

use crate::cargo_gate::{CargoRunner, CargoTest, require_tree_contains_regex, tree_contains_regex};
use crate::context::BaselineContext;
use anyhow::{Result, bail};

const LABEL: &str = "desktop-package-preflight-check";
const NATIVE_PACKAGING: &str = "native-packaging";

const DESKTOP_NATIVE_PACKAGING_TESTS: &[&str] = &["menu_tray", "packaging"];

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    println!("{LABEL}: run: native-track-boundary");
    crate::native_track_boundary::run()?;

    let runner = CargoRunner::without_target_dir(ctx.root(), LABEL)?;
    check_default_desktop_tree_excludes_tauri(&runner)?;
    check_desktop_native_tree_includes_runtime_surface(&runner)?;

    runner.run_check("deve_desktop", None, true)?;
    runner.run_test(&CargoTest {
        package: "deve_desktop",
        features: None,
        no_default_features: true,
        lib: false,
        test_target: None,
        filter: None,
    })?;
    runner.run_check("deve_desktop", Some(NATIVE_PACKAGING), false)?;
    for filter in DESKTOP_NATIVE_PACKAGING_TESTS {
        runner.run_test(&CargoTest {
            package: "deve_desktop",
            features: Some(NATIVE_PACKAGING),
            no_default_features: false,
            lib: false,
            test_target: None,
            filter: Some(filter),
        })?;
    }

    ctx.ok();
    Ok(())
}

fn check_default_desktop_tree_excludes_tauri(runner: &CargoRunner) -> Result<()> {
    let tree = runner.cargo_tree("deve_desktop", None, true)?;
    if tree_contains_regex(LABEL, &tree, r"(^| )tauri v")? {
        bail!("{LABEL}: default desktop dependency tree must remain no-Tauri");
    }
    Ok(())
}

fn check_desktop_native_tree_includes_runtime_surface(runner: &CargoRunner) -> Result<()> {
    let tree = runner.cargo_tree("deve_desktop", Some(NATIVE_PACKAGING), false)?;
    require_tree_contains_regex(
        LABEL,
        &tree,
        r"(^| )tauri v",
        "desktop native-packaging tree must include tauri",
    )?;
    require_tree_contains_regex(
        LABEL,
        &tree,
        r"(^| )tauri-build v",
        "desktop native-packaging tree must include tauri-build",
    )?;
    require_tree_contains_regex(
        LABEL,
        &tree,
        r"(^| )tray-icon v",
        "desktop native-packaging tree must include tray-icon",
    )?;
    require_tree_contains_regex(
        LABEL,
        &tree,
        r"(^| )tauri-runtime-wry v",
        "desktop native-packaging tree must include tauri-runtime-wry",
    )
}
