//! plan_ref: infra

use crate::cargo_gate::{CargoRunner, CargoTest, require_tree_contains_regex, tree_contains_regex};
use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

const LABEL: &str = "native-packaging-gate-check";
const NATIVE_PACKAGING: &str = "native-packaging";
const TARGET_DIR: &str = "target/native-packaging-gate";

const REQUIRED_LOCK_ENTRIES: &[(&str, &str)] = &[
    (
        r#"name = "tauri""#,
        "native-packaging dependency spike must lock tauri",
    ),
    (
        r#"name = "tauri-build""#,
        "native-packaging dependency spike must lock tauri-build",
    ),
    (
        r#"name = "tray-icon""#,
        "desktop native-packaging menu/tray binding must lock tray-icon",
    ),
    (
        r#"name = "tauri-runtime-wry""#,
        "native-packaging runtime entrypoint must lock tauri-runtime-wry",
    ),
];

const DESKTOP_NATIVE_PACKAGING_TESTS: &[&str] = &[
    "process_runtime",
    "service_entrypoint",
    "service_bootstrap",
    "tauri_bootstrap",
    "menu_tray",
    "packaging",
];

const MOBILE_NATIVE_PACKAGING_TESTS: &[&str] = &["packaging", "mobile_embedded_backend"];

const DESKTOP_NATIVE_TREE_REQUIREMENTS: &[(&str, &str)] = &[
    (
        r"(^| )tauri v",
        "desktop native-packaging feature must include tauri",
    ),
    (
        r"(^| )tauri-build v",
        "desktop native-packaging feature must include tauri-build",
    ),
    (
        r"(^| )tauri-runtime-wry v",
        "desktop native-packaging feature must include tauri-runtime-wry",
    ),
    (
        r"(^| )tray-icon v",
        "desktop native-packaging feature must include tray-icon",
    ),
];

const MOBILE_NATIVE_TREE_REQUIREMENTS: &[(&str, &str)] = &[
    (
        r"(^| )tauri v",
        "mobile native-packaging feature must include tauri",
    ),
    (
        r"(^| )tauri-build v",
        "mobile native-packaging feature must include tauri-build",
    ),
    (
        r"(^| )tauri-runtime-wry v",
        "mobile native-packaging feature must include tauri-runtime-wry",
    ),
];

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    println!("{LABEL}: run: native-track-boundary");
    crate::native_track_boundary::run()?;

    check_static_boundaries(&ctx)?;
    check_lock_entries(ctx.root())?;

    let runner = CargoRunner::with_target_dir(ctx.root(), LABEL, TARGET_DIR)?;
    check_default_dependency_tree_excludes_tauri(&runner, "deve_desktop", "desktop")?;
    check_default_dependency_tree_excludes_tauri(&runner, "deve_mobile", "mobile")?;
    check_native_packaging_tree_includes_runtime_surface(
        &runner,
        "deve_desktop",
        DESKTOP_NATIVE_TREE_REQUIREMENTS,
    )?;
    check_native_packaging_tree_includes_runtime_surface(
        &runner,
        "deve_mobile",
        MOBILE_NATIVE_TREE_REQUIREMENTS,
    )?;
    run_cargo_checks_and_tests(&runner)?;

    check_lock_entries(ctx.root())?;
    ctx.ok();
    Ok(())
}

fn check_static_boundaries(ctx: &BaselineContext) -> Result<()> {
    ctx.contains("docs/plan/17_tech_stack.md", "LocalBackend")?;
    ctx.contains("docs/plan/17_tech_stack.md", "RemoteBrowser")?;
    ctx.contains("docs/features/15_release.md", "Native 双模式")?;
    ctx.contains("docs/dev-runbook.md", "Native Shell Modes")?;
    ctx.contains(
        "scripts/check-desktop-native-session-package-smoke.sh",
        "DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED",
    )?;
    ctx.contains(
        "scripts/check-desktop-native-session-package-smoke.sh",
        "desktop-native-session-smoke: ok",
    )
}

fn check_lock_entries(root: &Path) -> Result<()> {
    let lock = fs::read_to_string(root.join("Cargo.lock"))
        .with_context(|| format!("{LABEL}: failed to read Cargo.lock"))?;
    for (needle, message) in REQUIRED_LOCK_ENTRIES {
        if !lock.contains(needle) {
            bail!("{LABEL}: {message}");
        }
    }
    Ok(())
}

fn check_default_dependency_tree_excludes_tauri(
    runner: &CargoRunner,
    package: &str,
    surface: &str,
) -> Result<()> {
    let tree = runner.cargo_tree(package, None, true)?;
    if tree_contains_regex(LABEL, &tree, r"(^| )tauri v")? {
        bail!("{LABEL}: default {surface} dependency tree must remain no-Tauri");
    }
    Ok(())
}

fn check_native_packaging_tree_includes_runtime_surface(
    runner: &CargoRunner,
    package: &str,
    requirements: &[(&str, &str)],
) -> Result<()> {
    let tree = runner.cargo_tree(package, Some(NATIVE_PACKAGING), false)?;
    for (pattern, message) in requirements {
        require_tree_contains_regex(LABEL, &tree, pattern, message)?;
    }
    Ok(())
}

fn run_cargo_checks_and_tests(runner: &CargoRunner) -> Result<()> {
    runner.run_check("deve_desktop", None, true)?;
    runner.run_check("deve_mobile", None, true)?;
    runner.run_check("deve_mobile", Some(NATIVE_PACKAGING), false)?;

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
    for filter in MOBILE_NATIVE_PACKAGING_TESTS {
        runner.run_test(&CargoTest {
            package: "deve_mobile",
            features: Some(NATIVE_PACKAGING),
            no_default_features: false,
            lib: false,
            test_target: None,
            filter: Some(filter),
        })?;
    }
    runner.run_test(&CargoTest {
        package: "deve_cli",
        features: None,
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("native_session"),
    })
}
