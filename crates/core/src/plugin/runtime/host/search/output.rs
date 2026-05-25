//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
use regex::Regex;
use rhai::EvalAltResult;
use std::path::Path;

use super::scope;

pub(super) fn collect_grep_matches(
    re: &Regex,
    path: &Path,
    root: &Path,
    content: &str,
    results: &mut Vec<String>,
    max_results: usize,
) -> Result<(), Box<EvalAltResult>> {
    let rel = scope::relative_search_path(root, path)?;
    for (i, line) in content.lines().enumerate() {
        if results.len() >= max_results {
            break;
        }
        if re.is_match(line) {
            let truncated: String = line.chars().take(200).collect();
            results.push(format!("{}:{}:{}", rel, i + 1, truncated));
        }
    }
    Ok(())
}

pub(super) fn format_file_results(
    pattern: &str,
    matches: &[String],
    max_results: usize,
) -> Result<String, Box<EvalAltResult>> {
    if matches.is_empty() {
        return Ok(format!("No files matching '{pattern}'"));
    }
    let count = matches.len();
    let suffix = if count >= max_results {
        format!("\n... (truncated at {max_results})")
    } else {
        String::new()
    };
    Ok(format!(
        "Found {count} file(s):\n{}{suffix}",
        matches.join("\n")
    ))
}

pub(super) fn format_grep_results(
    pattern: &str,
    results: &[String],
    max_results: usize,
) -> Result<String, Box<EvalAltResult>> {
    if results.is_empty() {
        return Ok(format!("No matches for '{pattern}'"));
    }
    let count = results.len();
    let suffix = if count >= max_results {
        format!("\n... (truncated at {max_results})")
    } else {
        String::new()
    };
    Ok(format!(
        "Found {count} match(es):\n{}{suffix}",
        results.join("\n")
    ))
}
