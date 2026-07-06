//! plan_ref: infra

use anyhow::Result;
use std::path::Path;

pub fn run(root: &Path, label: &str, package: &str, filter: &str) -> Result<()> {
    run_with_lib(root, label, package, filter, false)
}

pub fn run_lib(root: &Path, label: &str, package: &str, filter: &str) -> Result<()> {
    run_with_target(root, label, package, filter, true, None)
}

pub fn run_integration(
    root: &Path,
    label: &str,
    package: &str,
    test_target: &str,
    filter: &str,
) -> Result<()> {
    run_with_target(root, label, package, filter, false, Some(test_target))
}

fn run_with_lib(root: &Path, label: &str, package: &str, filter: &str, lib: bool) -> Result<()> {
    run_with_target(root, label, package, filter, lib, None)
}

fn run_with_target(
    root: &Path,
    label: &str,
    package: &str,
    filter: &str,
    lib: bool,
    test_target: Option<&str>,
) -> Result<()> {
    let runner = crate::cargo_gate::CargoRunner::without_target_dir(root, label)?;
    runner.run_test(&crate::cargo_gate::CargoTest {
        package,
        features: None,
        no_default_features: false,
        lib,
        test_target,
        filter: Some(filter),
    })
}
