//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

const LABEL: &str = "native-target-host-evidence-check";
const DEFAULT_REPORT: &str = "docs/report/native-target-host-evidence-template.md";
const PROCESS_RUNTIME_BOUNDARY_FIELD: &str = "Process runtime boundary: default no-Tauri closed; Desktop LocalBackend controlled child-process; Mobile child-process closed";

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
    PROCESS_RUNTIME_BOUNDARY_FIELD,
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
    use super::{PROCESS_RUNTIME_BOUNDARY_FIELD, has_exact_line, validate_report};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn target_line_requires_exact_match() {
        let template_line = "Target: Desktop macOS | Desktop Windows | Mobile iOS";

        assert!(!has_exact_line(template_line, "Target: Desktop macOS"));
        assert!(has_exact_line(
            "Target: Desktop macOS",
            "Target: Desktop macOS"
        ));
    }

    #[test]
    fn report_accepts_exact_process_runtime_boundary() {
        let report = write_report(&report_with_boundary(PROCESS_RUNTIME_BOUNDARY_FIELD));

        validate_report(&report.root, &report.path).expect("current process boundary should pass");
    }

    #[test]
    fn report_rejects_inexact_process_runtime_boundary() {
        let report = write_report(&report_with_boundary(
            "Process runtime boundary: Desktop LocalBackend controlled child-process",
        ));

        let err = validate_report(&report.root, &report.path)
            .expect_err("missing Mobile/no-Tauri boundary must fail closed");

        assert!(err.to_string().contains(PROCESS_RUNTIME_BOUNDARY_FIELD));
    }

    fn report_with_boundary(boundary: &str) -> String {
        format!(
            "# Native Target-host Evidence - test\n\n\
             Target: Local diagnostic\n\n\
             Workflow run: N/A\n\n\
             Host OS: test\n\n\
             Tool versions:\n\n\
             Commands:\n\n\
             Command results:\n\n\
             Artifact paths:\n\n\
             Install result: N/A\n\n\
             Startup result: N/A\n\n\
             {boundary}\n\n\
             Native authority writes: closed\n\n\
             Conclusion: diagnostic-only\n"
        )
    }

    fn write_report(content: &str) -> TempReport {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("deve-native-target-host-evidence-test-{unique}"));
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("evidence.md");
        fs::write(&path, content).expect("write temp evidence");
        TempReport { root, path }
    }

    struct TempReport {
        root: PathBuf,
        path: PathBuf,
    }

    impl Drop for TempReport {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
