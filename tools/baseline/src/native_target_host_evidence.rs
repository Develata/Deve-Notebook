//! plan_ref: infra

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

const LABEL: &str = "native-target-host-evidence-check";
const DEFAULT_REPORT: &str = "docs/report/native-target-host-evidence-template.md";

const REQUIRED_FIELDS: &[&str] = &[
    "# Native Target-host Evidence",
    "Target:",
    "Workflow run:",
    "Host OS:",
    "Tool versions:",
    "Commands:",
    "Command results:",
    "Artifact paths:",
    "Install result:",
    "Startup result:",
    "Process runtime gate: closed",
    "Native authority writes: closed",
    "Conclusion:",
];

const DESKTOP_REQUIRED_FIELDS: &[&str] = &[
    "desktop_preflight=",
    "process_gate=",
    "invalid_startup_request=",
    "invalid_installer_request=",
    "package_build=",
    "startup_smoke=",
    "native_session_smoke=",
    "installer_smoke=",
];

pub fn run(args: &[String]) -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    let reports = if args.is_empty() {
        vec![PathBuf::from(DEFAULT_REPORT)]
    } else {
        args.iter().map(PathBuf::from).collect()
    };

    for report in reports {
        let path = if report.is_absolute() {
            report
        } else {
            ctx.root().join(report)
        };
        validate_report(ctx.root(), &path)?;
    }

    println!("{LABEL}: ok");
    Ok(())
}

fn validate_report(root: &Path, path: &Path) -> Result<()> {
    if !path.is_file() {
        return fail(format!(
            "missing evidence file: {}",
            display_path(root, path)
        ));
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("{LABEL}: failed to read {}", display_path(root, path)))?;
    for field in REQUIRED_FIELDS {
        require_contains(root, path, &content, field)?;
    }

    if has_exact_line(&content, "Target: Desktop macOS")
        || has_exact_line(&content, "Target: Desktop Windows")
    {
        for field in DESKTOP_REQUIRED_FIELDS {
            require_contains(root, path, &content, field)?;
        }
    }

    Ok(())
}

fn require_contains(root: &Path, path: &Path, content: &str, text: &str) -> Result<()> {
    if content.contains(text) {
        Ok(())
    } else {
        fail(format!("missing '{text}' in {}", display_path(root, path)))
    }
}

fn has_exact_line(content: &str, expected: &str) -> bool {
    content.lines().any(|line| line == expected)
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn fail<T>(message: impl AsRef<str>) -> Result<T> {
    bail!("{LABEL}: {}", message.as_ref())
}

#[cfg(test)]
mod tests {
    use super::has_exact_line;

    #[test]
    fn target_line_requires_exact_match() {
        let template_line = "Target: Desktop macOS | Desktop Windows | Mobile iOS";

        assert!(!has_exact_line(template_line, "Target: Desktop macOS"));
        assert!(has_exact_line(
            "Target: Desktop macOS",
            "Target: Desktop macOS"
        ));
    }
}
