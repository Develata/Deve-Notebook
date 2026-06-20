//! plan_ref: infra

use anyhow::{Context, Result, bail};
use regex::Regex;
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

    pub fn absent_optional(&self, rel: &str, text: &str) -> Result<()> {
        let path = self.root.join(rel);
        if !path.try_exists().with_context(|| {
            format!(
                "{}: failed to check whether {} exists",
                self.label,
                display_rel(rel)
            )
        })? {
            return Ok(());
        }
        self.absent(rel, text)
    }

    pub fn absent_tree(&self, rel: &str, text: &str) -> Result<()> {
        self.scan_tree(rel, &TreeScan::default(), |path, content| {
            if content.contains(text) {
                bail!(
                    "{}: unexpected '{}' in {}",
                    self.label,
                    text,
                    display_path(path)
                )
            } else {
                Ok(())
            }
        })
    }

    pub fn absent_tree_skip_tests(&self, rel: &str, text: &str) -> Result<()> {
        self.scan_tree(rel, &TreeScan::default().skip_tests(), |path, content| {
            if content.contains(text) {
                bail!(
                    "{}: unexpected '{}' in {}",
                    self.label,
                    text,
                    display_path(path)
                )
            } else {
                Ok(())
            }
        })
    }

    pub fn regex_absent(&self, rel: &str, pattern: &str) -> Result<()> {
        let regex = compile_regex(self.label, pattern)?;
        let content = self.read(rel)?;
        if regex.is_match(&content) {
            bail!(
                "{}: pattern '{}' matched in {}",
                self.label,
                pattern,
                display_rel(rel)
            )
        } else {
            Ok(())
        }
    }

    pub fn regex_absent_tree(
        &self,
        rel: &str,
        pattern: &str,
        include_ext: Option<&str>,
        skip_suffixes: &[&str],
    ) -> Result<()> {
        let regex = compile_regex(self.label, pattern)?;
        let scan = TreeScan::default()
            .include_ext(include_ext)
            .skip_suffixes(skip_suffixes);
        self.scan_tree(rel, &scan, |path, content| {
            if regex.is_match(content) {
                bail!(
                    "{}: pattern '{}' matched in {}",
                    self.label,
                    pattern,
                    display_path(path)
                )
            } else {
                Ok(())
            }
        })
    }

    pub fn css_number_lt(&self, rel: &str, left: &str, right: &str) -> Result<()> {
        let content = self.read(rel)?;
        let left_value = css_number_value(&content, left)
            .with_context(|| format!("{}: missing token {left} in {rel}", self.label))?;
        let right_value = css_number_value(&content, right)
            .with_context(|| format!("{}: missing token {right} in {rel}", self.label))?;
        if left_value < right_value {
            Ok(())
        } else {
            bail!(
                "{}: {} ({}) must be below {} ({})",
                self.label,
                left,
                left_value,
                right,
                right_value
            )
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

    pub fn check_scripts_listed(&self, rel: &str) -> Result<()> {
        let content = self.read(rel)?;
        let mut scripts = Vec::new();
        for entry in fs::read_dir(self.root.join("scripts"))? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if path.is_file() && name.starts_with("check-") && name.ends_with(".sh") {
                scripts.push(format!("scripts/{name}"));
            }
        }
        scripts.sort();
        for script in scripts {
            if !content.contains(&script) {
                bail!("{}: missing '{}' in {}", self.label, script, rel);
            }
        }
        Ok(())
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
        let status = self.git(git_check_ignore_args(rel))?;
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

    pub fn cargo_test(&self, package: &str, filter: &str) -> Result<()> {
        crate::cargo_test::run(&self.root, self.label, package, filter)
    }

    pub fn ok(&self) {
        println!("{}: ok", self.label);
    }

    fn read(&self, rel: &str) -> Result<String> {
        fs::read_to_string(self.root.join(rel))
            .with_context(|| format!("{}: failed to read {}", self.label, display_rel(rel)))
    }

    fn scan_tree<F>(&self, rel: &str, scan: &TreeScan<'_>, mut check: F) -> Result<()>
    where
        F: FnMut(&Path, &str) -> Result<()>,
    {
        let root = self.root.join(rel);
        scan_tree_files(&root, scan, &mut |path| {
            let content = fs::read_to_string(path).with_context(|| {
                format!("{}: failed to read {}", self.label, display_path(path))
            })?;
            check(path, &content)
        })
        .with_context(|| format!("{}: failed to scan {}", self.label, display_rel(rel)))
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

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn compile_regex(label: &str, pattern: &str) -> Result<Regex> {
    Regex::new(pattern).with_context(|| format!("{label}: invalid regex '{pattern}'"))
}

#[derive(Default)]
struct TreeScan<'a> {
    include_ext: Option<&'a str>,
    skip_tests: bool,
    skip_suffixes: &'a [&'a str],
}

impl<'a> TreeScan<'a> {
    fn include_ext(mut self, include_ext: Option<&'a str>) -> Self {
        self.include_ext = include_ext;
        self
    }

    fn skip_tests(mut self) -> Self {
        self.skip_tests = true;
        self
    }

    fn skip_suffixes(mut self, skip_suffixes: &'a [&'a str]) -> Self {
        self.skip_suffixes = skip_suffixes;
        self
    }

    fn should_scan(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        if let Some(ext) = self.include_ext
            && path.extension().and_then(|value| value.to_str()) != Some(ext)
        {
            return false;
        }
        if self.skip_tests && is_test_file(path) {
            return false;
        }
        let normalized = path.to_string_lossy().replace('\\', "/");
        !self
            .skip_suffixes
            .iter()
            .any(|suffix| normalized.ends_with(suffix))
    }
}

fn scan_tree_files<F>(root: &Path, scan: &TreeScan<'_>, check: &mut F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    if root.is_file() {
        if scan.should_scan(root) {
            check(root)?;
        }
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_tree_files(&path, scan, check)?;
        } else if scan.should_scan(&path) {
            check(&path)?;
        }
    }
    Ok(())
}

fn is_test_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    matches!(name, "tests.rs")
        || name.ends_with("_test.rs")
        || name.ends_with("_tests.rs")
        || name.ends_with("_test.js")
        || name.ends_with("_tests.js")
}

fn css_number_value(content: &str, token: &str) -> Option<u64> {
    for line in content.lines() {
        let trimmed = line.trim();
        let Some(suffix) = trimmed.strip_prefix(token) else {
            continue;
        };
        let suffix = suffix.trim_start();
        let Some(suffix) = suffix.strip_prefix(':') else {
            continue;
        };
        let suffix = suffix.trim_start();
        let digits: String = suffix
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }
    None
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

fn git_check_ignore_args(rel: &str) -> [&str; 4] {
    ["check-ignore", "-q", "--no-index", rel]
}

#[cfg(test)]
mod tests {
    use super::{case_block, css_number_value, git_check_ignore_args, is_test_file};
    use std::path::Path;

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

    #[test]
    fn css_number_value_parses_registry_token() {
        assert_eq!(css_number_value("--z-editor: 0;\n", "--z-editor"), Some(0));
        assert_eq!(
            css_number_value("--z-toast: 120;\n", "--z-toast"),
            Some(120)
        );
    }

    #[test]
    fn test_file_filter_matches_rust_test_names() {
        assert!(is_test_file(Path::new("components/foo_tests.rs")));
        assert!(is_test_file(Path::new("components/foo_test.rs")));
        assert!(is_test_file(Path::new("components/tests.rs")));
        assert!(!is_test_file(Path::new("components/foo.rs")));
    }

    #[test]
    fn git_not_ignored_checks_ignore_rules_for_tracked_paths() {
        assert_eq!(
            git_check_ignore_args("Cargo.lock"),
            ["check-ignore", "-q", "--no-index", "Cargo.lock"]
        );
    }
}
