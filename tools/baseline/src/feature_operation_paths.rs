//! plan_ref:
//!   - 20_operations_catalog#opid-catalog

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const LABEL: &str = "feature-operation-path-check";
const SCAN_DIRS: [&str; 2] = ["docs/features/operations", "docs/acceptance-cases"];
const CHECKED_EXTENSIONS: [&str; 11] = [
    "rs", "sh", "md", "yml", "toml", "json", "css", "html", "lisp", "tsv", "js",
];

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new(LABEL)?;
    let refs = collect_path_refs(ctx.root())?;
    let mut failures = 0usize;

    for rel in refs {
        if should_skip_ref(&rel) || !has_checked_prefix(&rel) {
            continue;
        }

        let abs = ctx.root().join(&rel);
        if rel.ends_with('/') {
            if !abs.is_dir() {
                eprintln!("{LABEL}: missing directory: {rel}");
                failures += 1;
            }
            continue;
        }

        if has_checked_extension(&rel) && !abs.exists() {
            eprintln!("{LABEL}: missing file: {rel}");
            failures += 1;
        }
    }

    if failures > 0 {
        bail!("{LABEL}: {failures} missing path reference(s)");
    }

    println!("{LABEL}: ok");
    Ok(())
}

fn collect_path_refs(root: &Path) -> Result<BTreeSet<String>> {
    let pattern = Regex::new(r"`((?:apps|crates|scripts|docs|\.github)/[^` ]+)`")?;
    let mut refs = BTreeSet::new();
    let mut files = Vec::new();

    for rel_dir in SCAN_DIRS {
        collect_markdown_files(&root.join(rel_dir), &mut files)
            .with_context(|| format!("{LABEL}: failed to scan {rel_dir}"))?;
    }

    for file in files {
        let content = fs::read_to_string(&file)
            .with_context(|| format!("{LABEL}: failed to read {}", display_path(&file)))?;
        for captures in pattern.captures_iter(&content) {
            let rel = captures[1].trim_end_matches([',', '.']).to_string();
            refs.insert(rel);
        }
    }

    Ok(refs)
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            files.push(path);
        }
    }
    Ok(())
}

fn should_skip_ref(rel: &str) -> bool {
    rel.contains('*')
        || rel.contains('{')
        || rel.contains('}')
        || rel.contains('[')
        || rel.contains(']')
        || rel.contains("${")
        || rel.contains('"')
        || (rel.contains('<') && rel.contains('>'))
}

fn has_checked_prefix(rel: &str) -> bool {
    rel.starts_with("apps/")
        || rel.starts_with("crates/")
        || rel.starts_with("scripts/")
        || rel.starts_with("docs/")
        || rel.starts_with(".github/")
}

fn has_checked_extension(rel: &str) -> bool {
    Path::new(rel)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| CHECKED_EXTENSIONS.contains(&ext))
}

fn display_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{has_checked_extension, should_skip_ref};

    #[test]
    fn skips_template_or_glob_refs() {
        assert!(should_skip_ref("scripts/check-*.sh"));
        assert!(should_skip_ref("docs/${chapter}.md"));
        assert!(should_skip_ref("docs/features/<name>.md"));
        assert!(should_skip_ref("docs/[draft].md"));
        assert!(!should_skip_ref("docs/features/operation-coverage.md"));
    }

    #[test]
    fn checks_expected_file_extensions_only() {
        assert!(has_checked_extension("apps/cli/src/main.rs"));
        assert!(has_checked_extension(
            "scripts/check-feature-operation-paths.sh"
        ));
        assert!(has_checked_extension("docs/features/operation-coverage.md"));
        assert!(!has_checked_extension("Dockerfile"));
        assert!(!has_checked_extension("docs/features/operations/"));
    }
}
