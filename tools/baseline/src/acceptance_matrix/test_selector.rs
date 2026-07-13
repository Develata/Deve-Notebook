//! Cargo test evidence selector validation.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use super::model::MatrixRow;
use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn validate_test_selector(
    root: &Path,
    row: &MatrixRow,
    catalog: &TestCatalog,
) -> Result<()> {
    let selector = TestSelector::parse(&row.evidence_ref).with_context(|| {
        format!(
            "acceptance-matrix: invalid test selector for {}",
            row.requirement_id
        )
    })?;
    let package = catalog.packages.get(&selector.package).with_context(|| {
        format!(
            "acceptance-matrix: {} references unknown package {}",
            row.requirement_id, selector.package
        )
    })?;
    let search_root = if let Some(target) = selector.test_target.as_deref() {
        package.test_targets.get(target).with_context(|| {
            format!(
                "acceptance-matrix: {} references unknown test target {} in {}",
                row.requirement_id, target, selector.package
            )
        })?
    } else {
        &package.root
    };
    if let Some(filter) = selector.filter.as_deref()
        && !git_visible_rust_sources_define(root, search_root, filter)?
    {
        bail!(
            "acceptance-matrix: {} test filter is not defined in package {}: {filter}",
            row.requirement_id,
            selector.package
        );
    }
    Ok(())
}

fn git_visible_rust_sources_define(root: &Path, search_root: &Path, filter: &str) -> Result<bool> {
    let relative = search_root.strip_prefix(root).with_context(|| {
        format!(
            "acceptance-matrix: test source escapes workspace: {}",
            search_root.display()
        )
    })?;
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-co", "--exclude-standard", "--"])
        .arg(relative)
        .output()
        .context("acceptance-matrix: failed to list test sources")?;
    if !output.status.success() {
        bail!("acceptance-matrix: git ls-files failed while validating tests");
    }
    let paths = String::from_utf8(output.stdout)
        .context("acceptance-matrix: git ls-files output was not UTF-8")?;
    let test_function = Regex::new(
        r"(?ms)#\[(?:tokio::)?test[^\]]*\][^\{;]*?\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
    )?;
    for rel in paths
        .lines()
        .filter(|line| Path::new(line).extension().and_then(|value| value.to_str()) == Some("rs"))
    {
        if fs::read_to_string(root.join(rel))
            .map(|content| {
                test_function
                    .captures_iter(&content)
                    .any(|capture| capture[1].contains(filter))
            })
            .unwrap_or(false)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug)]
struct TestSelector {
    package: String,
    test_target: Option<String>,
    filter: Option<String>,
}

impl TestSelector {
    fn parse(reference: &str) -> Result<Self> {
        let words: Vec<_> = reference.split_whitespace().collect();
        if words.first() != Some(&"cargo") || words.get(1) != Some(&"test") {
            bail!("test evidence must start with `cargo test`");
        }
        let separator = words
            .iter()
            .position(|word| *word == "--")
            .unwrap_or(words.len());
        let mut package = None;
        let mut test_target = None;
        let mut filter = None;
        let mut index = 2usize;
        while index < separator {
            match words[index] {
                "-p" | "--package" => {
                    index += 1;
                    package = Some(
                        words
                            .get(index)
                            .context("package option is missing its value")?
                            .to_string(),
                    );
                }
                "--test" => {
                    index += 1;
                    test_target = Some(
                        words
                            .get(index)
                            .context("test target option is missing its value")?
                            .to_string(),
                    );
                }
                "--lib" | "--locked" => {}
                value if value.starts_with('-') => {
                    bail!("unsupported cargo test option {value}")
                }
                value if filter.is_none() => filter = Some(value.to_string()),
                value => bail!("unexpected cargo test argument {value}"),
            }
            index += 1;
        }
        if filter.is_none() && test_target.is_none() {
            bail!("test evidence needs a filter or an explicit --test target");
        }
        Ok(Self {
            package: package.context("cargo test evidence requires -p/--package")?,
            test_target,
            filter,
        })
    }
}

#[derive(Debug)]
struct TestPackage {
    root: PathBuf,
    test_targets: BTreeMap<String, PathBuf>,
}

#[derive(Debug)]
pub(super) struct TestCatalog {
    packages: BTreeMap<String, TestPackage>,
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
    manifest_path: PathBuf,
    targets: Vec<CargoTarget>,
}

#[derive(Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
    src_path: PathBuf,
}

impl TestCatalog {
    pub(super) fn load(root: &Path) -> Result<Self> {
        let output = Command::new("cargo")
            .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
            .current_dir(root)
            .output()
            .context("acceptance-matrix: failed to run cargo metadata")?;
        if !output.status.success() {
            bail!("acceptance-matrix: cargo metadata failed");
        }
        let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
            .context("acceptance-matrix: cargo metadata was invalid")?;
        let mut packages = BTreeMap::new();
        for package in metadata.packages {
            let root = package
                .manifest_path
                .parent()
                .context("package manifest has no parent")?
                .to_path_buf();
            let test_targets = package
                .targets
                .into_iter()
                .filter(|target| target.kind.iter().any(|kind| kind == "test"))
                .map(|target| (target.name, target.src_path))
                .collect();
            packages.insert(package.name, TestPackage { root, test_targets });
        }
        Ok(Self { packages })
    }
}

#[cfg(test)]
mod tests {
    use super::TestSelector;

    #[test]
    fn test_selector_requires_package_and_understands_test_targets() {
        let unit =
            TestSelector::parse("cargo test -p deve_core typed_filter -- --nocapture").unwrap();
        assert_eq!(unit.package, "deve_core");
        assert_eq!(unit.filter.as_deref(), Some("typed_filter"));
        let integration = TestSelector::parse(
            "cargo test --package deve_core --test materialize_projection_test -- --nocapture",
        )
        .unwrap();
        assert_eq!(
            integration.test_target.as_deref(),
            Some("materialize_projection_test")
        );
        assert!(TestSelector::parse("cargo test -p missing").is_err());
    }
}
