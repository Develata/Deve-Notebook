//! plan_ref: infra

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LABEL: &str = "acceptance-bindings";
const ACCEPTANCE_DIR: &str = "docs/acceptance-cases";
const ACCEPTANCE_BINDINGS: &str = "docs/acceptance-bindings.tsv";
const FEATURE_OP_DIR: &str = "docs/features/operations";
const FEATURE_OP_COVERAGE: &str = "docs/features/operation-coverage.md";
const CODE_DIRS: [&str; 4] = ["crates", "apps", "tests", "scripts"];
const VALID_BINDINGS: [&str; 5] = [
    "manual-chrome",
    "manual-cli",
    "manual-doc",
    "manual-network",
    "manual-security",
];
const STALE_COMMANDS: [&str; 5] = [
    "deve dump --doc",
    "deve merge --peer",
    "deve auth decode-jwt",
    "deve api call",
    "cargo test -p deve_core path_normalize_structure -- --nocapture",
];

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    let acceptance_dir = ctx.root().join(ACCEPTANCE_DIR);
    if !acceptance_dir.is_dir() {
        bail!("ERROR: missing acceptance directory: {ACCEPTANCE_DIR}");
    }

    let case_ids = collect_case_ids(&acceptance_dir)?;
    let ordered_case_ids = ordered_case_ids(&case_ids);
    let mut errors = Vec::new();
    let manual_map = collect_manual_bindings(ctx.root(), &case_ids, &mut errors)?;
    let automated_map = collect_case_refs(ctx.root(), &CODE_DIRS, &ordered_case_ids)?;
    let feature_map = collect_feature_refs(ctx.root(), &ordered_case_ids)?;

    let mut automated = 0usize;
    let mut feature = 0usize;
    let mut manual = 0usize;
    let mut unbound = 0usize;

    for case_id in &case_ids {
        if automated_map.contains(case_id) {
            automated += 1;
        } else if feature_map.contains(case_id) {
            feature += 1;
        } else if manual_map.contains_key(case_id) {
            manual += 1;
        } else {
            println!("unbound case: {case_id}");
            unbound += 1;
        }
    }

    record_stale_commands(&acceptance_dir, &mut errors)?;
    for error in &errors {
        println!("ERROR: {error}");
    }

    println!("automated acceptance bindings: {automated}");
    println!("feature walkthrough bindings: {feature}");
    println!("manual acceptance bindings: {manual}");
    println!("unbound acceptance cases (soft): {unbound}");

    if errors.is_empty() {
        Ok(())
    } else {
        bail!("{LABEL}: {} error(s)", errors.len())
    }
}

fn collect_case_ids(acceptance_dir: &Path) -> Result<BTreeSet<String>> {
    let re = Regex::new(r"case_id: ([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*)")?;
    let mut ids = BTreeSet::new();
    let mut files = Vec::new();

    for entry in fs::read_dir(acceptance_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();

    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("{LABEL}: failed to read {}", display_path(&path)))?;
        for captures in re.captures_iter(&content) {
            ids.insert(captures[1].to_string());
        }
    }

    Ok(ids)
}

fn collect_manual_bindings(
    root: &Path,
    case_ids: &BTreeSet<String>,
    errors: &mut Vec<String>,
) -> Result<BTreeMap<String, String>> {
    let path = root.join(ACCEPTANCE_BINDINGS);
    let mut bindings = BTreeMap::new();
    if !path.is_file() {
        return Ok(bindings);
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("{LABEL}: failed to read {ACCEPTANCE_BINDINGS}"))?;
    for line in content.lines() {
        let Some(row) = parse_binding_row(line) else {
            continue;
        };

        if !case_ids.contains(row.case_id) {
            errors.push(format!(
                "acceptance binding references unknown case: {}",
                row.case_id
            ));
            continue;
        }
        if bindings.contains_key(row.case_id) {
            errors.push(format!("duplicate acceptance binding: {}", row.case_id));
            continue;
        }
        if !VALID_BINDINGS.contains(&row.binding) {
            errors.push(format!(
                "invalid acceptance binding type for {}: {}",
                row.case_id, row.binding
            ));
            continue;
        }

        let evidence_path = row.evidence.split('#').next().unwrap_or_default();
        if evidence_path.is_empty() || !root.join(evidence_path).is_file() {
            errors.push(format!(
                "acceptance binding evidence missing for {}: {}",
                row.case_id, row.evidence
            ));
            continue;
        }

        bindings.insert(row.case_id.to_string(), row.binding.to_string());
    }

    Ok(bindings)
}

fn collect_feature_refs(root: &Path, case_ids: &[String]) -> Result<BTreeSet<String>> {
    let mut targets = Vec::new();
    let coverage = root.join(FEATURE_OP_COVERAGE);
    if coverage.is_file() {
        targets.push(coverage);
    }
    let op_dir = root.join(FEATURE_OP_DIR);
    if op_dir.is_dir() {
        collect_files(&op_dir, &mut targets)?;
    }
    collect_refs_from_files(&targets, case_ids)
}

fn collect_case_refs(root: &Path, dirs: &[&str], case_ids: &[String]) -> Result<BTreeSet<String>> {
    let files = git_visible_files(root, dirs)?;
    collect_refs_from_files(&files, case_ids)
}

fn git_visible_files(root: &Path, dirs: &[&str]) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("ls-files")
        .arg("-co")
        .arg("--exclude-standard")
        .arg("--")
        .args(dirs)
        .output()
        .with_context(|| format!("{LABEL}: failed to run git ls-files"))?;
    if !output.status.success() {
        bail!("{LABEL}: git ls-files failed");
    }

    let stdout = String::from_utf8(output.stdout)
        .with_context(|| format!("{LABEL}: git ls-files output was not UTF-8"))?;
    Ok(stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| root.join(line))
        .collect())
}

fn collect_refs_from_files(files: &[PathBuf], case_ids: &[String]) -> Result<BTreeSet<String>> {
    let mut refs = BTreeSet::new();
    for path in files {
        if let Ok(content) = fs::read_to_string(path) {
            refs.extend(case_ids_in_content(&content, case_ids));
        }
    }
    Ok(refs)
}

fn record_stale_commands(acceptance_dir: &Path, errors: &mut Vec<String>) -> Result<()> {
    let mut files = Vec::new();
    collect_files(acceptance_dir, &mut files)?;
    let stale_field_re = Regex::new(r"--field (doc_id|last_op)")?;

    for path in files {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for pattern in STALE_COMMANDS {
            if content.contains(pattern) {
                errors.push(format!("stale acceptance command remains: {pattern}"));
            }
        }
        if stale_field_re.is_match(&content) {
            errors.push("stale acceptance dump --field command remains".to_string());
        }
    }
    Ok(())
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if root.is_file() {
        files.push(root.to_path_buf());
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(root)? {
        entries.push(entry?.path());
    }
    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn ordered_case_ids(case_ids: &BTreeSet<String>) -> Vec<String> {
    let mut ordered: Vec<_> = case_ids.iter().cloned().collect();
    ordered.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    ordered
}

fn case_ids_in_content(content: &str, case_ids: &[String]) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    let mut occupied = vec![false; content.len()];

    for case_id in case_ids {
        for (start, _) in content.match_indices(case_id) {
            let end = start + case_id.len();
            if occupied[start..end].iter().any(|used| *used) {
                continue;
            }
            refs.insert(case_id.clone());
            occupied[start..end].fill(true);
        }
    }

    refs
}

struct BindingRow<'a> {
    case_id: &'a str,
    binding: &'a str,
    evidence: &'a str,
}

fn parse_binding_row(line: &str) -> Option<BindingRow<'_>> {
    let mut parts = line.splitn(4, '|');
    let case_id = parts.next().unwrap_or_default().trim();
    let binding = parts.next().unwrap_or_default().trim();
    let evidence = parts.next().unwrap_or_default().trim();
    if case_id.is_empty() || case_id.starts_with('#') {
        return None;
    }
    Some(BindingRow {
        case_id,
        binding,
        evidence,
    })
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{case_ids_in_content, ordered_case_ids, parse_binding_row};
    use std::collections::BTreeSet;

    #[test]
    fn longer_case_ids_do_not_shadow_shorter_prefixes() {
        let ids = BTreeSet::from([
            "CMD-004".to_string(),
            "CMD-004A".to_string(),
            "CMD-004B".to_string(),
        ]);
        let ordered = ordered_case_ids(&ids);
        let refs = case_ids_in_content("case CMD-004A only", &ordered);

        assert!(refs.contains("CMD-004A"));
        assert!(!refs.contains("CMD-004"));
        assert!(!refs.contains("CMD-004B"));
    }

    #[test]
    fn parses_binding_rows_with_optional_notes() {
        let row = parse_binding_row("AUTH-004|manual-security|docs/features/09_auth.md|note")
            .expect("binding row");

        assert_eq!(row.case_id, "AUTH-004");
        assert_eq!(row.binding, "manual-security");
        assert_eq!(row.evidence, "docs/features/09_auth.md");
        assert!(parse_binding_row("# comment").is_none());
        assert!(parse_binding_row("").is_none());
    }
}
