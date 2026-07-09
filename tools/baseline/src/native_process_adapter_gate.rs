//! plan_ref: infra

use crate::cargo_gate::{CargoRunner, CargoTest};
use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

const LABEL: &str = "native-process-adapter-gate-check";
const DEFAULT_TARGET_DIR: &str = "target/native-process-gate";
const RUN_NATIVE_PACKAGING_TESTS_ENV: &str =
    "DEVE_NATIVE_PROCESS_ADAPTER_RUN_NATIVE_PACKAGING_TESTS";
const RUN_DESKTOP_NATIVE_PACKAGING_TESTS_ENV: &str =
    "DEVE_NATIVE_PROCESS_ADAPTER_RUN_DESKTOP_NATIVE_PACKAGING_TESTS";

const PROCESS_RUNTIME_ALLOWED: &[&str] = &[
    "apps/desktop/src/process_runtime.rs",
    "apps/desktop/src/process_runtime/launcher.rs",
    "apps/desktop/src/process_runtime/process_group.rs",
    "apps/desktop/src/process_runtime/process_group/windows.rs",
];

const REQUIRED_CARGO_TESTS: &[CargoTest] = &[
    CargoTest {
        package: "deve_core",
        features: None,
        no_default_features: false,
        lib: true,
        test_target: None,
        filter: Some("native_adapter::process_test"),
    },
    CargoTest {
        package: "deve_desktop",
        features: None,
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("desktop_default_build_defers_real_process_adapter"),
    },
    CargoTest {
        package: "deve_mobile",
        features: None,
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("mobile_default_build_defers_real_process_adapter"),
    },
    CargoTest {
        package: "deve_cli",
        features: None,
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("native_session"),
    },
];

const MOBILE_NATIVE_PACKAGING_TESTS: &[CargoTest] = &[CargoTest {
    package: "deve_mobile",
    features: Some("native-packaging"),
    no_default_features: false,
    lib: false,
    test_target: None,
    filter: Some("mobile_embedded_backend"),
}];

const DESKTOP_NATIVE_PACKAGING_TESTS: &[CargoTest] = &[
    CargoTest {
        package: "deve_desktop",
        features: Some("native-packaging"),
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("service_entrypoint"),
    },
    CargoTest {
        package: "deve_desktop",
        features: Some("native-packaging"),
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("service_bootstrap"),
    },
    CargoTest {
        package: "deve_desktop",
        features: Some("native-packaging"),
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("process_runtime_test"),
    },
];

const PROCESS_OBSERVATION_TESTS: &[CargoTest] = &[
    CargoTest {
        package: "deve_desktop",
        features: None,
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("process_observation"),
    },
    CargoTest {
        package: "deve_mobile",
        features: None,
        no_default_features: false,
        lib: false,
        test_target: None,
        filter: Some("process_observation"),
    },
];

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    println!("{LABEL}: run: native-track-boundary");
    crate::native_track_boundary::run()?;
    run_tsv(&ctx, include_str!("specs/native_process_adapter_gate.tsv"))?;
    check_no_process_runtime_leak(ctx.root())?;
    run_cargo_tests(ctx.root())?;
    ctx.ok();
    Ok(())
}

fn check_no_process_runtime_leak(root: &Path) -> Result<()> {
    let process_regex =
        Regex::new(r"(^|[^[:alnum:]_])(std::process|Command::new|tokio::process|\.spawn\()")
            .with_context(|| format!("{LABEL}: invalid process runtime regex"))?;
    for line in matching_lines(
        root,
        &[
            "apps/desktop/src",
            "apps/mobile/src",
            "crates/core/src/native_adapter",
        ],
        &Scan::new().skip_dirs(&["gen", "target", "node_modules", "dist"]),
        &process_regex,
    )? {
        if PROCESS_RUNTIME_ALLOWED.contains(&line.rel.as_str()) {
            continue;
        }
        fail(format!(
            "native process runtime is only allowed in the Desktop post-gate runtime spike: {}",
            line.display()
        ))?;
    }

    if root.join("apps/mobile/src/process_runtime.rs").exists() {
        fail("mobile process runtime must remain closed")?;
    }
    let mobile_lib = fs::read_to_string(root.join("apps/mobile/src/lib.rs"))
        .with_context(|| format!("{LABEL}: failed to read apps/mobile/src/lib.rs"))?;
    let mobile_process_mod = Regex::new(r"mod[[:space:]]+process_runtime[[:space:]]*;")
        .with_context(|| format!("{LABEL}: invalid mobile process module regex"))?;
    if mobile_process_mod.is_match(&mobile_lib) {
        fail("mobile process runtime must remain closed")?;
    }

    Ok(())
}

fn run_cargo_tests(root: &Path) -> Result<()> {
    let runner = CargoRunner::with_target_dir(root, LABEL, DEFAULT_TARGET_DIR)?;

    for test in REQUIRED_CARGO_TESTS {
        runner.run_test(test)?;
    }

    if should_run_native_packaging_tests()? {
        for test in MOBILE_NATIVE_PACKAGING_TESTS {
            runner.run_test(test)?;
        }

        if should_run_desktop_native_packaging_tests()? {
            for test in DESKTOP_NATIVE_PACKAGING_TESTS {
                runner.run_test(test)?;
            }
        } else {
            println!("{LABEL}: skip Desktop native-packaging tests for scoped target-host run");
        }
    } else {
        println!("{LABEL}: skip native-packaging tests for release scope without native artifacts");
    }

    for test in PROCESS_OBSERVATION_TESTS {
        runner.run_test(test)?;
    }

    Ok(())
}

fn should_run_native_packaging_tests() -> Result<bool> {
    binary_env_flag(RUN_NATIVE_PACKAGING_TESTS_ENV, true)
}

fn should_run_desktop_native_packaging_tests() -> Result<bool> {
    binary_env_flag(RUN_DESKTOP_NATIVE_PACKAGING_TESTS_ENV, true)
}

fn binary_env_flag(name: &str, default: bool) -> Result<bool> {
    let value = std::env::var(name).unwrap_or_else(|_| if default { "1" } else { "0" }.to_string());
    match value.as_str() {
        "1" | "true" | "TRUE" | "yes" | "YES" => Ok(true),
        "0" | "false" | "FALSE" | "no" | "NO" => Ok(false),
        _ => bail!("{LABEL}: invalid {name}: {value}"),
    }
}

fn matching_lines(
    root: &Path,
    rel_roots: &[&str],
    scan: &Scan<'_>,
    regex: &Regex,
) -> Result<Vec<MatchedLine>> {
    let mut matches = Vec::new();
    for rel_root in rel_roots {
        for path in collect_files(root, rel_root, scan)? {
            let Some(content) = read_text_if_nonbinary(&path)? else {
                continue;
            };
            let rel = display_path(root, &path);
            for (index, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    matches.push(MatchedLine {
                        rel: rel.clone(),
                        line_no: index + 1,
                        text: line.to_string(),
                    });
                }
            }
        }
    }
    Ok(matches)
}

fn collect_files(root: &Path, rel: &str, scan: &Scan<'_>) -> Result<Vec<PathBuf>> {
    let start = root.join(rel);
    let mut files = Vec::new();
    collect_files_inner(&start, scan, &mut files)?;
    Ok(files)
}

fn collect_files_inner(path: &Path, scan: &Scan<'_>, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(path).with_context(|| format!("{LABEL}: failed to scan {path:?}"))? {
        let entry = entry?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("{LABEL}: failed to inspect {:?}", entry.path()))?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            if scan.should_skip_dir(&path) {
                continue;
            }
            collect_files_inner(&path, scan, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn read_text_if_nonbinary(path: &Path) -> Result<Option<String>> {
    let bytes = fs::read(path).with_context(|| format!("{LABEL}: failed to read {path:?}"))?;
    if bytes.contains(&0) {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
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

#[derive(Clone, Copy, Default)]
struct Scan<'a> {
    skip_dirs: &'a [&'a str],
}

impl<'a> Scan<'a> {
    fn new() -> Self {
        Self::default()
    }

    fn skip_dirs(mut self, skip_dirs: &'a [&'a str]) -> Self {
        self.skip_dirs = skip_dirs;
        self
    }

    fn should_skip_dir(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            return false;
        };
        self.skip_dirs.contains(&name)
    }
}

struct MatchedLine {
    rel: String,
    line_no: usize,
    text: String,
}

impl MatchedLine {
    fn display(&self) -> String {
        format!("{}:{}:{}", self.rel, self.line_no, self.text)
    }
}
