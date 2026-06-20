//! plan_ref: infra

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct BaselineContext {
    root: PathBuf,
    label: &'static str,
}

impl BaselineContext {
    pub fn new(label: &'static str) -> Result<Self> {
        Ok(Self {
            root: repo_root()?,
            label,
        })
    }

    pub fn contains(&self, rel: &str, text: &str) -> Result<()> {
        let content = self.read(rel)?;
        if content.contains(text) {
            Ok(())
        } else {
            bail!("{}: missing '{}' in {}", self.label, text, display_rel(rel))
        }
    }

    pub fn absent(&self, rel: &str, text: &str) -> Result<()> {
        let content = self.read(rel)?;
        if content.contains(text) {
            bail!(
                "{}: unexpected '{}' in {}",
                self.label,
                text,
                display_rel(rel)
            )
        } else {
            Ok(())
        }
    }

    pub fn before(&self, rel: &str, before: &str, after: &str) -> Result<()> {
        let content = self.read(rel)?;
        let before_line = first_line_no(&content, before)
            .with_context(|| format!("{}: missing '{}' in {}", self.label, before, rel))?;
        let after_line = first_line_no(&content, after)
            .with_context(|| format!("{}: missing '{}' in {}", self.label, after, rel))?;
        if before_line < after_line {
            Ok(())
        } else {
            bail!(
                "{}: '{}' must appear before '{}' in {}",
                self.label,
                before,
                after,
                display_rel(rel)
            )
        }
    }

    pub fn case_contains(&self, acceptance: &str, case_id: &str, text: &str) -> Result<()> {
        let content = self.read(acceptance)?;
        let block = case_block(&content, case_id)
            .with_context(|| format!("{}: missing case block {case_id}", self.label))?;
        if block.contains(text) {
            Ok(())
        } else {
            bail!("{}: missing '{}' in {}", self.label, text, case_id)
        }
    }

    pub fn git_tracked(&self, rel: &str) -> Result<()> {
        let status = self.git(["ls-files", "--error-unmatch", rel])?;
        if status.success() {
            Ok(())
        } else {
            bail!(
                "{}: git ls-files --error-unmatch failed for {}",
                self.label,
                display_rel(rel)
            )
        }
    }

    pub fn git_not_ignored(&self, rel: &str) -> Result<()> {
        let status = self.git(["check-ignore", "-q", rel])?;
        match status.code() {
            Some(1) => Ok(()),
            _ if status.success() => bail!("{}: {} must not be ignored", self.label, rel),
            _ => bail!(
                "{}: git check-ignore failed for {}",
                self.label,
                display_rel(rel)
            ),
        }
    }

    pub fn ok(&self) {
        println!("{}: ok", self.label);
    }

    fn read(&self, rel: &str) -> Result<String> {
        fs::read_to_string(self.root.join(rel))
            .with_context(|| format!("{}: failed to read {}", self.label, display_rel(rel)))
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Result<std::process::ExitStatus> {
        let safe_directory = format!("safe.directory={}", self.root.display());
        Command::new("git")
            .arg("-c")
            .arg(safe_directory)
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .with_context(|| format!("{}: failed to run git", self.label))
    }
}

fn repo_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("failed to resolve repository root from CARGO_MANIFEST_DIR")
}

fn display_rel(rel: &str) -> String {
    rel.replace('\\', "/")
}

fn first_line_no(content: &str, text: &str) -> Option<usize> {
    content
        .lines()
        .position(|line| line.contains(text))
        .map(|index| index + 1)
}

fn case_block(content: &str, case_id: &str) -> Result<String> {
    let mut in_case = false;
    let mut block = String::new();

    for line in content.lines() {
        if let Some(current_case_id) = line_case_id(line) {
            if current_case_id == case_id {
                in_case = true;
            } else if in_case {
                break;
            }
        }
        if in_case {
            block.push_str(line);
            block.push('\n');
        }
    }

    if block.is_empty() {
        bail!("case block not found")
    }
    Ok(block)
}

fn line_case_id(line: &str) -> Option<&str> {
    line.trim_start().strip_prefix("- case_id: ").map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::case_block;

    #[test]
    fn baseline_case_block_stops_at_next_case() {
        let content = "- case_id: STORE-001\n  steps:\n    - run: one\n- case_id: STORE-002\n  steps:\n    - run: two\n";
        let block = case_block(content, "STORE-001").expect("case block");

        assert!(block.contains("run: one"));
        assert!(!block.contains("run: two"));
    }

    #[test]
    fn baseline_case_block_reports_missing_case() {
        assert!(case_block("- case_id: STORE-001\n", "STORE-999").is_err());
    }

    #[test]
    fn baseline_case_block_requires_exact_case_id() {
        let content = "- case_id: STORE-010\n  steps:\n    - run: wrong\n- case_id: STORE-01\n  steps:\n    - run: right\n";
        let block = case_block(content, "STORE-01").expect("case block");

        assert!(block.contains("run: right"));
        assert!(!block.contains("run: wrong"));
    }

    #[test]
    fn baseline_case_block_stops_at_prefixed_next_case_id() {
        let content = "- case_id: STORE-01\n  steps:\n    - run: one\n- case_id: STORE-010\n  steps:\n    - run: two\n";
        let block = case_block(content, "STORE-01").expect("case block");

        assert!(block.contains("run: one"));
        assert!(!block.contains("run: two"));
    }
}
