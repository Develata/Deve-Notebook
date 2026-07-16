//! Parsers for acceptance cases, operation coverage, and the matrix TSV.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::model::{FlowCase, HEADER, MATRIX_PATH, MatrixRow};
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(super) fn read_matrix(root: &Path) -> Result<Vec<MatrixRow>> {
    let content = fs::read_to_string(root.join(MATRIX_PATH))
        .with_context(|| format!("acceptance-matrix: failed to read {MATRIX_PATH}"))?;
    parse_matrix(&content)
}

pub(super) fn parse_matrix(content: &str) -> Result<Vec<MatrixRow>> {
    let mut lines = content.lines();
    let header = lines.next().unwrap_or_default();
    let actual: Vec<_> = header.split('\t').collect();
    if actual != HEADER {
        bail!(
            "acceptance-matrix: invalid header; expected {}",
            HEADER.join(" | ")
        );
    }
    let mut rows = Vec::new();
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != HEADER.len() {
            bail!(
                "acceptance-matrix: line {} has {} fields, expected {}",
                offset + 2,
                fields.len(),
                HEADER.len()
            );
        }
        if fields.iter().any(|value| value.trim() != *value) {
            bail!(
                "acceptance-matrix: line {} contains padded fields",
                offset + 2
            );
        }
        rows.push(MatrixRow::from_fields(&fields));
    }
    Ok(rows)
}

pub(super) fn collect_case_ids(root: &Path) -> Result<BTreeSet<String>> {
    let case_re = Regex::new(r"case_id: ([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*)")?;
    let mut ids = BTreeSet::new();
    let mut files = Vec::new();
    collect_markdown(&root.join("docs/acceptance-cases"), &mut files)?;
    files.sort();
    for path in files {
        let content = fs::read_to_string(&path)?;
        ids.extend(
            case_re
                .captures_iter(&content)
                .map(|capture| capture[1].to_string()),
        );
    }
    Ok(ids)
}

pub(super) fn collect_flow_cases(root: &Path) -> Result<BTreeSet<FlowCase>> {
    let content = fs::read_to_string(root.join("docs/features/operation-coverage.md"))?;
    let flow_re = Regex::new(r"flow\.[a-z0-9.-]+")?;
    let case_re = Regex::new(r"[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+")?;
    let mut relations = BTreeSet::new();
    for line in content.lines().filter(|line| line.starts_with("| `flow.")) {
        let flow_id = flow_re
            .find(line)
            .map(|found| found.as_str().to_string())
            .context("acceptance-matrix: operation coverage row missing flow ID")?;
        let columns: Vec<_> = line.split('|').collect();
        let cases = columns
            .get(3)
            .context("acceptance-matrix: operation coverage row missing case column")?;
        let mut count = 0usize;
        for found in case_re.find_iter(cases) {
            relations.insert(FlowCase {
                flow_id: flow_id.clone(),
                case_id: found.as_str().to_string(),
            });
            count += 1;
        }
        if count == 0 {
            bail!("acceptance-matrix: {flow_id} has no acceptance cases");
        }
    }
    Ok(relations)
}

fn collect_markdown(root: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_markdown(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_matrix;
    use crate::acceptance_matrix::model::HEADER;

    #[test]
    fn matrix_parser_requires_exact_tsv_width() {
        let good = format!("{}\n{}\n", HEADER.join("\t"), vec!["x"; 13].join("\t"));
        assert_eq!(parse_matrix(&good).unwrap().len(), 1);
        assert!(parse_matrix(&format!("{}\nx\n", HEADER.join("\t"))).is_err());
    }
}
