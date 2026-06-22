//! plan_ref: infra

use anyhow::Result;
use std::path::Path;

pub fn run(root: &Path, label: &str, package: &str, filter: &str) -> Result<()> {
    let runner = crate::cargo_gate::CargoRunner::without_target_dir(root, label)?;
    runner.run_test(&crate::cargo_gate::CargoTest {
        package,
        features: None,
        no_default_features: false,
        lib: false,
        filter: Some(filter),
    })
}
