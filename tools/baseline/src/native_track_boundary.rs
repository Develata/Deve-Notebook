//! plan_ref: infra

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::{Context, Result, bail};
use regex::{Regex, RegexBuilder};
use std::fs;
use std::path::{Path, PathBuf};

const LABEL: &str = "native-track-boundary-check";

const ICONS: &[(&str, &str)] = &[
    (
        "apps/desktop/icons/icon.png",
        "missing desktop Tauri icon: apps/desktop/icons/icon.png",
    ),
    (
        "apps/desktop/icons/icon.ico",
        "missing desktop Tauri Windows icon: apps/desktop/icons/icon.ico",
    ),
    (
        "apps/desktop/icons/icon.icns",
        "missing desktop Tauri macOS icon: apps/desktop/icons/icon.icns",
    ),
    (
        "apps/mobile/icons/icon.png",
        "missing mobile Tauri icon: apps/mobile/icons/icon.png",
    ),
    (
        "apps/mobile/icons/icon.ico",
        "missing mobile Tauri Windows icon: apps/mobile/icons/icon.ico",
    ),
    (
        "apps/mobile/icons/icon.icns",
        "missing mobile Tauri macOS icon: apps/mobile/icons/icon.icns",
    ),
];

const MANIFEST_DEPENDENCIES: &[(&str, &str, &str)] = &[
    ("apps/desktop/Cargo.toml", "indexmap_1", "1.9.3"),
    ("apps/desktop/Cargo.toml", "tauri", "2.11.1"),
    ("apps/desktop/Cargo.toml", "tauri-build", "2.6.1"),
    ("apps/desktop/Cargo.toml", "windows-sys", "0.61.2"),
    ("apps/mobile/Cargo.toml", "indexmap_1", "1.9.3"),
    ("apps/mobile/Cargo.toml", "tauri", "2.11.1"),
    ("apps/mobile/Cargo.toml", "tauri-build", "2.6.1"),
];

const TAURI_IMPORT_ALLOWED: &[&str] = &[
    "apps/desktop/src/menu_tray.rs",
    "apps/desktop/src/main.rs",
    "apps/desktop/src/tauri_bootstrap/mod.rs",
    "apps/desktop/src/tauri_bootstrap/cookie.rs",
    "apps/desktop/src/tauri_entry/mod.rs",
    "apps/mobile/src/tauri_entry.rs",
    "apps/mobile/src/tauri_lifecycle.rs",
    "apps/mobile/src/embedded_backend/mod.rs",
    "apps/mobile/src/embedded_backend/cookie.rs",
    "apps/mobile/src/embedded_backend/generation.rs",
    "apps/mobile/src/embedded_backend/supervisor.rs",
    "apps/mobile/src/embedded_backend/supervisor_webview.rs",
    "apps/mobile/src/embedded_backend/supervisor_tests.rs",
    "apps/mobile/src/embedded_backend/supervisor_types.rs",
];

const PROCESS_RUNTIME_ALLOWED: &[&str] = &[
    "apps/desktop/src/process_runtime.rs",
    "apps/desktop/src/process_runtime/launcher.rs",
    "apps/desktop/src/process_runtime/process_group.rs",
    "apps/desktop/src/process_runtime/process_group/windows.rs",
];

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    run_tsv(&ctx, include_str!("specs/native_track_boundary.tsv"))?;
    check_manifest_dependencies(ctx.root())?;
    check_icons_exist(ctx.root())?;
    check_no_packaging_dependency_leak(ctx.root())?;
    check_no_windows_sys_dependency_leak(ctx.root())?;
    check_no_process_runtime_leak(ctx.root())?;
    ctx.ok();
    Ok(())
}

fn check_manifest_dependencies(root: &Path) -> Result<()> {
    for (file, dep, version) in MANIFEST_DEPENDENCIES {
        let content = read_required(root, file)?;
        let prefix = format!("{dep} =");
        let line = content
            .lines()
            .find(|line| line.trim_start().starts_with(&prefix))
            .with_context(|| format!("{LABEL}: missing dependency '{dep}' in {file}"))?;
        if !line.contains(&format!("version = \"{version}\"")) {
            return fail(format!(
                "dependency '{dep}' must pin version {version} in {file}"
            ));
        }
        if !line.contains("optional = true") {
            return fail(format!("dependency '{dep}' must stay optional in {file}"));
        }
        if !line.contains("default-features = false") {
            return fail(format!(
                "dependency '{dep}' must disable default features in {file}"
            ));
        }
    }
    Ok(())
}

fn check_icons_exist(root: &Path) -> Result<()> {
    for (icon, message) in ICONS {
        if !root.join(icon).is_file() {
            return fail(*message);
        }
    }
    Ok(())
}

fn check_no_packaging_dependency_leak(root: &Path) -> Result<()> {
    let tauri_manifest = RegexBuilder::new(
        r#"(^[[:space:]]*["']?(tauri|tauri-build)["']?[[:space:]]*=|package[[:space:]]*=[[:space:]]*["'](tauri|tauri-build)["']|^[[:space:]]*\[[^]]*(dependencies|dev-dependencies|build-dependencies)\.["']?(tauri|tauri-build)["']?[[:space:]]*\])"#,
    )
    .case_insensitive(true)
    .multi_line(true)
    .build()
    .with_context(|| format!("{LABEL}: invalid tauri manifest regex"))?;

    let cargo_tomls = collect_cargo_tomls(root)?;
    for cargo_toml in cargo_tomls {
        if cargo_toml.file_name().and_then(|value| value.to_str()) != Some("Cargo.toml") {
            continue;
        }
        let content = read_path_required(&cargo_toml)?;
        if tauri_manifest.is_match(&content) {
            let rel = display_path(root, &cargo_toml);
            if matches!(
                rel.as_str(),
                "apps/desktop/Cargo.toml" | "apps/mobile/Cargo.toml"
            ) {
                continue;
            }
            return fail(format!(
                "native packaging dependency is not allowed yet: {rel}"
            ));
        }
    }

    let import_regex =
        Regex::new(r"(^|[^[:alnum:]_])((use[[:space:]]+tauri(::|[[:space:];,{]))|tauri::)")
            .with_context(|| format!("{LABEL}: invalid tauri import regex"))?;
    for line in matching_lines(
        root,
        &["apps", "crates"],
        &Scan::new().skip_dirs(&["gen", "target", "node_modules", "dist"]),
        &import_regex,
    )? {
        if TAURI_IMPORT_ALLOWED.contains(&line.rel.as_str()) {
            continue;
        }
        return fail(format!(
            "native packaging runtime import outside native shell binding: {}",
            line.display()
        ));
    }

    Ok(())
}

fn check_no_windows_sys_dependency_leak(root: &Path) -> Result<()> {
    let manifest_regex = RegexBuilder::new(
        r#"(^[[:space:]]*["']?windows-sys["']?[[:space:]]*=|package[[:space:]]*=[[:space:]]*["']windows-sys["']|^[[:space:]]*\[[^]]*(dependencies|dev-dependencies|build-dependencies)\.["']?windows-sys["']?[[:space:]]*\])"#,
    )
    .case_insensitive(true)
    .multi_line(true)
    .build()
    .with_context(|| format!("{LABEL}: invalid windows-sys manifest regex"))?;

    for cargo_toml in collect_cargo_tomls(root)? {
        let content = read_path_required(&cargo_toml)?;
        if !manifest_regex.is_match(&content) {
            continue;
        }
        let rel = display_path(root, &cargo_toml);
        if rel == "apps/desktop/Cargo.toml" {
            continue;
        }
        return fail(format!(
            "windows-sys dependency is only allowed in Desktop native-packaging scope: {rel}"
        ));
    }

    let import_regex = Regex::new(r"(^|[^[:alnum:]_])windows_sys::")
        .with_context(|| format!("{LABEL}: invalid windows-sys import regex"))?;
    for line in matching_lines(
        root,
        &["apps", "crates"],
        &Scan::new().skip_dirs(&["gen", "target", "node_modules", "dist"]),
        &import_regex,
    )? {
        if line.rel == "apps/desktop/src/process_runtime/process_group/windows.rs"
            || line
                .rel
                .starts_with("apps/desktop/src/process_runtime/process_group/windows/")
        {
            continue;
        }
        return fail(format!(
            "windows-sys runtime import outside Desktop process adapter: {}",
            line.display()
        ));
    }

    Ok(())
}

fn collect_cargo_tomls(root: &Path) -> Result<Vec<PathBuf>> {
    let mut manifests = Vec::new();
    let workspace_manifest = root.join("Cargo.toml");
    if workspace_manifest.is_file() {
        manifests.push(workspace_manifest);
    }

    let scan = Scan::new().skip_dirs(&["target", "node_modules", "gen", "dist"]);
    for rel in ["apps", "crates", "tools"] {
        for path in collect_files(root, rel, &scan)? {
            if path.file_name().and_then(|value| value.to_str()) == Some("Cargo.toml") {
                manifests.push(path);
            }
        }
    }
    Ok(manifests)
}

fn check_no_process_runtime_leak(root: &Path) -> Result<()> {
    let process_regex =
        Regex::new(r"(^|[^[:alnum:]_])(std::process|Command::new|tokio::process|\.spawn\()")
            .with_context(|| format!("{LABEL}: invalid process runtime regex"))?;
    for line in matching_lines(
        root,
        &["apps/desktop/src", "apps/mobile/src"],
        &Scan::new(),
        &process_regex,
    )? {
        if PROCESS_RUNTIME_ALLOWED.contains(&line.rel.as_str()) {
            continue;
        }
        return fail(format!(
            "native process runtime is only allowed in the Desktop post-gate runtime spike: {}",
            line.display()
        ));
    }

    if root.join("apps/mobile/src/process_runtime.rs").exists() {
        return fail("mobile process runtime must remain closed");
    }
    let mobile_lib = read_required(root, "apps/mobile/src/lib.rs")?;
    let mobile_process_mod = Regex::new(r"mod[[:space:]]+process_runtime[[:space:]]*;")
        .with_context(|| format!("{LABEL}: invalid mobile process module regex"))?;
    if mobile_process_mod.is_match(&mobile_lib) {
        return fail("mobile process runtime must remain closed");
    }

    let authority_regex = Regex::new(
        r"(^|[^[:alnum:]_])(ledger|vault|projection_workspace|ProjectionWorkspace|source_control|search|GitMirror|NoteGit|std::fs|OpenOptions|File::create|File::options)",
    )
    .with_context(|| format!("{LABEL}: invalid process authority regex"))?;
    let authority_roots = [
        "apps/desktop/src/process_runtime.rs",
        "apps/desktop/src/process_runtime",
    ];
    if !matching_lines(root, &authority_roots, &Scan::new(), &authority_regex)?.is_empty() {
        return fail("desktop process runtime must remain authority-free");
    }

    Ok(())
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
    let start = if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    };
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

fn read_required(root: &Path, rel: &str) -> Result<String> {
    read_path_required(&root.join(rel)).with_context(|| format!("{LABEL}: failed to read {rel}"))
}

fn read_path_required(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("{LABEL}: failed to read {path:?}"))
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
