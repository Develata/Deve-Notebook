//! plan_ref: infra

use anyhow::{Context, Result, bail};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub fn run(root: &Path, label: &str, package: &str, filter: &str) -> Result<()> {
    println!("{label}: run: cargo test -p {package} {filter} -- --nocapture");
    let output = Command::new("cargo")
        .arg("test")
        .arg("-p")
        .arg(package)
        .arg(filter)
        .arg("--")
        .arg("--nocapture")
        .current_dir(root)
        .output()
        .with_context(|| format!("{label}: failed to run cargo test"))?;

    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;

    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        bail!("{label}: cargo test failed for {package} {filter}");
    }
    let total = executed_test_count(&combined);
    if total < 1 {
        bail!("{label}: expected at least one executed test for filter '{filter}'");
    }
    Ok(())
}

fn executed_test_count(output: &str) -> u64 {
    output
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("running ")?;
            let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
            digits.parse::<u64>().ok()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::executed_test_count;

    #[test]
    fn counts_all_cargo_test_running_lines() {
        let output = "running 0 tests\nrunning 2 tests\nrunning 1 test\n";
        assert_eq!(executed_test_count(output), 3);
    }
}
