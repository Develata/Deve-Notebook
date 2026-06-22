//! plan_ref: infra

use anyhow::{Context, Result, bail};
use regex::RegexBuilder;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct CargoRunner {
    root: PathBuf,
    label: String,
    cargo: String,
    target_dir: Option<String>,
}

impl CargoRunner {
    pub fn without_target_dir(root: &Path, label: &str) -> Result<Self> {
        Ok(Self {
            root: root.to_path_buf(),
            label: label.to_string(),
            cargo: select_cargo_bin(root, label)?,
            target_dir: None,
        })
    }

    pub fn with_target_dir(root: &Path, label: &str, default_target_dir: &str) -> Result<Self> {
        Ok(Self {
            root: root.to_path_buf(),
            label: label.to_string(),
            cargo: select_cargo_bin(root, label)?,
            target_dir: Some(
                env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| default_target_dir.to_string()),
            ),
        })
    }

    pub fn cargo_tree(
        &self,
        package: &str,
        features: Option<&str>,
        no_default_features: bool,
    ) -> Result<String> {
        let mut args = vec![
            "tree".to_string(),
            "--locked".to_string(),
            "-p".to_string(),
            package.to_string(),
        ];
        push_feature_args(&mut args, features, no_default_features);
        self.run_captured(args, false, &format!("cargo tree failed for {package}"))
    }

    pub fn run_check(
        &self,
        package: &str,
        features: Option<&str>,
        no_default_features: bool,
    ) -> Result<()> {
        let mut args = vec!["check".to_string()];
        self.push_target_dir(&mut args);
        args.extend([
            "--locked".to_string(),
            "-p".to_string(),
            package.to_string(),
        ]);
        push_feature_args(&mut args, features, no_default_features);
        self.run_captured(args, true, &format!("cargo check failed for {package}"))?;
        Ok(())
    }

    pub fn run_test(&self, test: &CargoTest<'_>) -> Result<()> {
        let mut args = vec!["test".to_string()];
        self.push_target_dir(&mut args);
        args.extend([
            "--locked".to_string(),
            "-p".to_string(),
            test.package.to_string(),
        ]);
        push_feature_args(&mut args, test.features, test.no_default_features);
        if test.lib {
            args.push("--lib".to_string());
        }
        if let Some(filter) = test.filter {
            args.push(filter.to_string());
        }
        args.push("--".to_string());
        args.push("--nocapture".to_string());

        let combined = self.run_captured(
            args,
            true,
            &format!(
                "cargo test failed for {} {}",
                test.package,
                test.filter.unwrap_or("<all>")
            ),
        )?;
        let total = executed_test_count(&combined);
        if total < 1 {
            bail!(
                "{}: expected at least one executed test for {} {}",
                self.label,
                test.package,
                test.filter.unwrap_or("<all>")
            );
        }
        Ok(())
    }

    fn push_target_dir(&self, args: &mut Vec<String>) {
        if let Some(target_dir) = self.target_dir.as_ref() {
            args.push("--target-dir".to_string());
            args.push(target_dir.clone());
        }
    }

    fn run_captured(
        &self,
        args: Vec<String>,
        write_output: bool,
        failure_message: &str,
    ) -> Result<String> {
        println!("{}: run: {} {}", self.label, self.cargo, args.join(" "));
        let output = Command::new(&self.cargo)
            .args(&args)
            .current_dir(&self.root)
            .output()
            .with_context(|| format!("{}: failed to run cargo", self.label))?;

        if write_output || !output.status.success() {
            io::stdout().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;
        }

        let combined = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            bail!("{}: {}", self.label, failure_message);
        }
        Ok(combined)
    }
}

pub struct CargoTest<'a> {
    pub package: &'a str,
    pub features: Option<&'a str>,
    pub no_default_features: bool,
    pub lib: bool,
    pub filter: Option<&'a str>,
}

pub fn executed_test_count(output: &str) -> u64 {
    output
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("running ")?;
            let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
            digits.parse::<u64>().ok()
        })
        .sum()
}

pub fn tree_contains_regex(label: &str, tree: &str, pattern: &str) -> Result<bool> {
    Ok(RegexBuilder::new(pattern)
        .multi_line(true)
        .build()
        .with_context(|| format!("{label}: invalid cargo tree regex '{pattern}'"))?
        .is_match(tree))
}

pub fn require_tree_contains_regex(
    label: &str,
    tree: &str,
    pattern: &str,
    message: &str,
) -> Result<()> {
    if tree_contains_regex(label, tree, pattern)? {
        Ok(())
    } else {
        bail!("{label}: {message}")
    }
}

fn push_feature_args(args: &mut Vec<String>, features: Option<&str>, no_default_features: bool) {
    if no_default_features {
        args.push("--no-default-features".to_string());
    }
    if let Some(features) = features {
        args.push("--features".to_string());
        args.push(features.to_string());
    }
}

fn select_cargo_bin(root: &Path, label: &str) -> Result<String> {
    if let Ok(cargo_bin) = env::var("CARGO_BIN") {
        if command_exists(&cargo_bin) {
            return Ok(cargo_bin);
        }
        bail!("{label}: configured CARGO_BIN '{cargo_bin}' was not found");
    }

    if let Ok(cargo) = env::var("CARGO") {
        if command_exists(&cargo) {
            return Ok(cargo);
        }
        bail!("{label}: configured CARGO '{cargo}' was not found");
    }

    let candidates: &[&str] = if is_wsl_mounted_workspace(root) {
        &["cargo.exe", "cargo"]
    } else {
        &["cargo", "cargo.exe"]
    };

    for candidate in candidates {
        if command_exists(candidate) {
            return Ok((*candidate).to_string());
        }
    }

    bail!("{label}: cargo command not found")
}

fn command_exists(candidate: &str) -> bool {
    Command::new(candidate)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn is_wsl_mounted_workspace(root: &Path) -> bool {
    let root_display = root.to_string_lossy().replace('\\', "/");
    if !root_display.starts_with("/mnt/") {
        return false;
    }
    fs::read_to_string("/proc/version")
        .map(|version| version.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{executed_test_count, tree_contains_regex};

    #[test]
    fn counts_all_cargo_test_running_lines() {
        let output = "running 0 tests\nrunning 2 tests\nrunning 1 test\n";
        assert_eq!(executed_test_count(output), 3);
    }

    #[test]
    fn cargo_tree_match_uses_line_boundaries() {
        let tree = "deve_desktop v0.0.1\n├── tauri v2.11.1\n└── tray-icon v0.21.2\n";

        assert!(tree_contains_regex("test", tree, r"(^| )tauri v").unwrap());
        assert!(tree_contains_regex("test", tree, r"(^| )tray-icon v").unwrap());
        assert!(!tree_contains_regex("test", tree, r"(^| )tauri-build v").unwrap());
    }
}
