//! plan_ref:
//!   - 17_tech_stack#canonical-rust-toolchain

use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(crate) const EXACT_TOOLCHAIN: &str = "1.97.0";
const WORKSPACE_MSRV: &str = "1.97";
pub(crate) const RUST_ACTION_REF: &str =
    "dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88";
const CARGO_AUDIT_VERSION_PROBE: &str =
    r#"run: test "$(cargo-audit --version)" = "cargo-audit 0.22.2""#;
const WORKFLOWS: [&str; 7] = [
    ".github/workflows/acceptance-aggregate.yml",
    ".github/workflows/check.yml",
    ".github/workflows/docker-smoke.yml",
    ".github/workflows/native-target-host.yml",
    ".github/workflows/release-candidate.yml",
    ".github/workflows/release-native.yml",
    ".github/workflows/release.yml",
];
const CARGO_AUDIT_WORKFLOWS: [&str; 2] = [
    ".github/workflows/docker-smoke.yml",
    ".github/workflows/release-candidate.yml",
];
const MEMBER_MANIFESTS: [&str; 6] = [
    "apps/cli/Cargo.toml",
    "apps/desktop/Cargo.toml",
    "apps/mobile/Cargo.toml",
    "apps/web/Cargo.toml",
    "crates/core/Cargo.toml",
    "tools/baseline/Cargo.toml",
];

pub(super) fn check(root: &Path) -> Result<()> {
    check_toolchain_file(root)?;
    check_cargo_metadata(root)?;
    check_workflows(root)?;
    check_cargo_audit_probes(root)?;
    require_token(root, "Dockerfile", "FROM rust:1.97.0-bookworm AS build-env")?;
    require_token(
        root,
        "scripts/check-mobile-android-shell-package-build.sh",
        "RUST_TOOLCHAIN=\"${DEVE_MOBILE_ANDROID_RUST_TOOLCHAIN:-1.97.0}\"",
    )?;
    require_token(
        root,
        "scripts/check-desktop-linux-apptainer-slurm.sh",
        "/.rustup/toolchains/1.97.0-x86_64-unknown-linux-gnu",
    )?;
    Ok(())
}

fn check_cargo_audit_probes(root: &Path) -> Result<()> {
    for workflow in CARGO_AUDIT_WORKFLOWS {
        check_cargo_audit_probe_content(workflow, &read(root, workflow)?)?;
    }
    Ok(())
}

fn check_cargo_audit_probe_content(workflow: &str, content: &str) -> Result<()> {
    let matches = content
        .lines()
        .filter(|line| line.trim() == CARGO_AUDIT_VERSION_PROBE)
        .count();
    if matches != 1 {
        bail!(
            "release-baseline-check: {workflow} must contain exactly one direct cargo-audit 0.22.2 version probe"
        );
    }
    Ok(())
}

fn check_toolchain_file(root: &Path) -> Result<()> {
    let content = read(root, "rust-toolchain.toml")?;
    check_toolchain_content(&content)
}

fn check_cargo_metadata(root: &Path) -> Result<()> {
    let workspace = read(root, "Cargo.toml")?;
    let package = parse_table(&workspace, "workspace.package", "Cargo.toml")?;
    require_scalar(&package, "rust-version", WORKSPACE_MSRV, "Cargo.toml")?;
    for manifest in MEMBER_MANIFESTS {
        let content = read(root, manifest)?;
        let package = parse_table(&content, "package", manifest)?;
        if package.get("rust-version.workspace").map(String::as_str) != Some("true") {
            bail!(
                "release-baseline-check: {manifest} [package] must inherit rust-version.workspace = true"
            );
        }
    }
    Ok(())
}

fn check_toolchain_content(content: &str) -> Result<()> {
    let toolchain = parse_table(content, "toolchain", "rust-toolchain.toml")?;
    let keys = toolchain
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected = ["channel", "components", "profile", "targets"]
        .into_iter()
        .collect::<BTreeSet<_>>();
    if keys != expected {
        bail!(
            "release-baseline-check: rust-toolchain.toml [toolchain] keys must be {expected:?}, found {keys:?}"
        );
    }
    require_scalar(
        &toolchain,
        "channel",
        EXACT_TOOLCHAIN,
        "rust-toolchain.toml",
    )?;
    require_scalar(&toolchain, "profile", "minimal", "rust-toolchain.toml")?;
    require_string_array(
        &toolchain,
        "components",
        &["rustfmt", "clippy"],
        "rust-toolchain.toml",
    )?;
    require_string_array(
        &toolchain,
        "targets",
        &["wasm32-unknown-unknown"],
        "rust-toolchain.toml",
    )
}

fn check_workflows(root: &Path) -> Result<()> {
    let workflow_root = root.join(".github/workflows");
    let mut checked = BTreeSet::new();
    for entry in
        fs::read_dir(&workflow_root).with_context(|| format!("read {}", workflow_root.display()))?
    {
        let path = entry?.path();
        if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("read workflow {}", path.display()))?;
        let has_toolchain_action = content.contains("dtolnay/rust-toolchain@");
        if workflow_uses_rust_commands(&content) && !has_toolchain_action {
            bail!(
                "release-baseline-check: workflow {} runs Rust commands without the pinned toolchain action",
                path.display()
            );
        }
        if !has_toolchain_action {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .context("workflow filename must be valid UTF-8")?;
        let workflow = format!(".github/workflows/{name}");
        check_workflow_content(&workflow, &content)?;
        checked.insert(workflow);
    }
    for required in WORKFLOWS {
        if !checked.contains(required) {
            bail!("release-baseline-check: required Rust workflow {required} is missing its pin");
        }
    }
    Ok(())
}

fn workflow_uses_rust_commands(content: &str) -> bool {
    content.lines().any(|line| {
        line.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(|token| matches!(token, "cargo" | "rustc" | "rustup"))
    })
}

fn check_workflow_content(workflow: &str, content: &str) -> Result<()> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut action_count = 0usize;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(action_ref) = trimmed.strip_prefix("uses: dtolnay/rust-toolchain@") else {
            continue;
        };
        action_count += 1;
        let actual_ref = format!("dtolnay/rust-toolchain@{action_ref}");
        if actual_ref != RUST_ACTION_REF {
            bail!(
                "release-baseline-check: {workflow} uses mutable or unexpected Rust action {actual_ref}"
            );
        }
        let (step_start, step_end) = workflow_step_bounds(&lines, index).with_context(|| {
            format!("release-baseline-check: {workflow} Rust action must belong to one YAML step")
        })?;
        let expected_name = format!("- name: Install Rust {EXACT_TOOLCHAIN}");
        if lines[step_start].trim() != expected_name {
            bail!(
                "release-baseline-check: {workflow} Rust action step must be labelled '{expected_name}'"
            );
        }
        let uses_indent = leading_spaces(lines[index]);
        let with_indices = (index + 1..step_end)
            .filter(|candidate| {
                leading_spaces(lines[*candidate]) == uses_indent
                    && lines[*candidate].trim() == "with:"
            })
            .collect::<Vec<_>>();
        let toolchains = with_indices
            .first()
            .into_iter()
            .flat_map(|with_index| {
                (*with_index + 1..step_end)
                    .take_while(|candidate| {
                        lines[*candidate].trim().is_empty()
                            || leading_spaces(lines[*candidate]) > uses_indent
                    })
                    .filter_map(|candidate| {
                        lines[candidate]
                            .trim()
                            .strip_prefix("toolchain:")
                            .map(parse_scalar)
                    })
            })
            .collect::<Vec<_>>();
        if with_indices.len() != 1 || toolchains != [EXACT_TOOLCHAIN] {
            bail!(
                "release-baseline-check: {workflow} Rust action must use exact toolchain {EXACT_TOOLCHAIN}"
            );
        }
    }
    if action_count == 0 {
        bail!("release-baseline-check: {workflow} has no Rust toolchain action");
    }
    Ok(())
}

fn workflow_step_bounds(lines: &[&str], uses_index: usize) -> Option<(usize, usize)> {
    let uses_indent = leading_spaces(lines[uses_index]);
    let start = (0..uses_index).rev().find(|candidate| {
        let line = lines[*candidate];
        !line.trim().is_empty()
            && leading_spaces(line) < uses_indent
            && line.trim_start().starts_with("- ")
    })?;
    let step_indent = leading_spaces(lines[start]);
    let end = (start + 1..lines.len())
        .find(|candidate| {
            let line = lines[*candidate];
            !line.trim().is_empty()
                && leading_spaces(line) == step_indent
                && line.trim_start().starts_with("- ")
        })
        .unwrap_or(lines.len());
    (uses_index < end).then_some((start, end))
}

fn leading_spaces(value: &str) -> usize {
    value.bytes().take_while(|byte| *byte == b' ').count()
}

fn parse_scalar(value: &str) -> &str {
    value.trim().trim_matches(['\'', '"'])
}

fn parse_table(content: &str, table: &str, scope: &str) -> Result<BTreeMap<String, String>> {
    let mut in_table = false;
    let mut seen_table = false;
    let mut assignments = BTreeMap::new();
    let header = format!("[{table}]");
    for raw_line in content.lines() {
        let line = strip_toml_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_table = line == header;
            if in_table {
                if seen_table {
                    bail!("release-baseline-check: duplicate [{table}] table in {scope}");
                }
                seen_table = true;
            }
            continue;
        }
        if !in_table {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("release-baseline-check: invalid assignment in {scope}"))?;
        let key = key.trim().to_owned();
        if assignments
            .insert(key.clone(), value.trim().to_owned())
            .is_some()
        {
            bail!("release-baseline-check: duplicate {key} in {scope} [{table}]");
        }
    }
    if !seen_table {
        bail!("release-baseline-check: missing [{table}] table in {scope}");
    }
    Ok(assignments)
}

fn strip_toml_comment(line: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quote == Some('"') {
            escaped = true;
            continue;
        }
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            continue;
        }
        if character == '#' && quote.is_none() {
            return &line[..index];
        }
    }
    line
}

fn require_scalar(
    assignments: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
    scope: &str,
) -> Result<()> {
    let actual = assignments
        .get(key)
        .and_then(|value| parse_toml_string(value));
    if actual != Some(expected) {
        bail!(
            "release-baseline-check: {scope} must define exactly {key} = \"{expected}\", found {actual:?}"
        );
    }
    Ok(())
}

fn require_string_array(
    assignments: &BTreeMap<String, String>,
    key: &str,
    expected: &[&str],
    scope: &str,
) -> Result<()> {
    let raw = assignments
        .get(key)
        .with_context(|| format!("release-baseline-check: missing {key} in {scope}"))?;
    let inner = raw
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .with_context(|| format!("release-baseline-check: {key} in {scope} must be an array"))?;
    let actual = inner
        .split(',')
        .filter(|value| !value.trim().is_empty())
        .map(|value| parse_toml_string(value).unwrap_or_default())
        .collect::<Vec<_>>();
    if actual != expected {
        bail!("release-baseline-check: {key} in {scope} must be {expected:?}, found {actual:?}");
    }
    Ok(())
}

fn parse_toml_string(value: &str) -> Option<&str> {
    value.trim().strip_prefix('"')?.strip_suffix('"')
}

fn require_token(root: &Path, relative: &str, token: &str) -> Result<()> {
    let content = read(root, relative)?;
    require_text(&content, token, relative)
}

fn require_text(content: &str, token: &str, scope: &str) -> Result<()> {
    if !content.contains(token) {
        bail!("release-baseline-check: missing '{token}' in {scope}");
    }
    Ok(())
}

fn read(root: &Path, relative: &str) -> Result<String> {
    let path = root.join(relative);
    fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_workflow(action: &str, toolchain: &str) -> String {
        format!(
            "      - name: Install Rust {EXACT_TOOLCHAIN}\n        uses: {action}\n        with:\n          toolchain: \"{toolchain}\"\n"
        )
    }

    fn exact_toolchain() -> &'static str {
        "[toolchain]\nchannel = \"1.97.0\"\nprofile = \"minimal\"\ncomponents = [\"rustfmt\", \"clippy\"]\ntargets = [\"wasm32-unknown-unknown\"]\n"
    }

    #[test]
    fn accepts_exact_pinned_action_and_toolchain() {
        check_workflow_content(
            "fixture.yml",
            &exact_workflow(RUST_ACTION_REF, EXACT_TOOLCHAIN),
        )
        .unwrap();
    }

    #[test]
    fn rejects_mutable_action_revision() {
        let error = check_workflow_content(
            "fixture.yml",
            &exact_workflow("dtolnay/rust-toolchain@stable", EXACT_TOOLCHAIN),
        )
        .unwrap_err();
        assert!(error.to_string().contains("mutable or unexpected"));
    }

    #[test]
    fn rejects_minor_only_toolchain_input() {
        let error = check_workflow_content("fixture.yml", &exact_workflow(RUST_ACTION_REF, "1.97"))
            .unwrap_err();
        assert!(error.to_string().contains("exact toolchain 1.97.0"));
    }

    #[test]
    fn rejects_toolchain_fields_in_the_wrong_table() {
        let fixture = exact_toolchain().replacen("[toolchain]", "[unrelated]", 1);
        let error = check_toolchain_content(&fixture).unwrap_err();
        assert!(error.to_string().contains("missing [toolchain]"));
    }

    #[test]
    fn rejects_commented_toolchain_fields() {
        let fixture = exact_toolchain().replace(
            "components = [\"rustfmt\", \"clippy\"]",
            "# components = [\"rustfmt\", \"clippy\"]",
        );
        let error = check_toolchain_content(&fixture).unwrap_err();
        assert!(error.to_string().contains("keys must be"));
    }

    #[test]
    fn rejects_toolchain_borrowed_from_the_next_step() {
        let fixture = format!(
            "      - name: Install Rust {EXACT_TOOLCHAIN}\n        uses: {RUST_ACTION_REF}\n      - name: Unrelated\n        with:\n          toolchain: \"{EXACT_TOOLCHAIN}\"\n"
        );
        let error = check_workflow_content("fixture.yml", &fixture).unwrap_err();
        assert!(error.to_string().contains("must use exact toolchain"));
    }

    #[test]
    fn rejects_label_borrowed_from_an_unrelated_step() {
        let fixture = format!(
            "      - name: Other Rust step\n        uses: {RUST_ACTION_REF}\n        with:\n          toolchain: \"{EXACT_TOOLCHAIN}\"\n      - name: Install Rust {EXACT_TOOLCHAIN}\n        run: rustc --version\n"
        );
        let error = check_workflow_content("fixture.yml", &fixture).unwrap_err();
        assert!(error.to_string().contains("must be labelled"));
    }

    #[test]
    fn detects_rust_commands_without_an_action() {
        assert!(workflow_uses_rust_commands(
            "steps:\n  - name: Check\n    run: cargo check --locked\n"
        ));
        assert!(!workflow_uses_rust_commands(
            "steps:\n  - name: Browser check\n    run: npm test\n"
        ));
    }

    #[test]
    fn cargo_audit_probe_requires_the_direct_binary() {
        check_cargo_audit_probe_content(
            "fixture.yml",
            &format!("      {CARGO_AUDIT_VERSION_PROBE}\n"),
        )
        .expect("direct cargo-audit probe");

        let error = check_cargo_audit_probe_content(
            "fixture.yml",
            r#"      run: test "$(cargo audit --version)" = "cargo-audit 0.22.2"
"#,
        )
        .expect_err("Cargo subcommand probe changes the version banner");
        assert!(error.to_string().contains("direct cargo-audit 0.22.2"));
    }
}
