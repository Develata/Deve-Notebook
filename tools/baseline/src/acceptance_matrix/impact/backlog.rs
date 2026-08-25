//! Honest executable-evidence backlog for acceptance cases.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use crate::acceptance_matrix::model::MatrixRow;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
struct BacklogReport {
    schema: u8,
    status: &'static str,
    document_only_total: usize,
    unautomated_total: usize,
    document_only_by_family: BTreeMap<String, Vec<String>>,
    unautomated_by_family: BTreeMap<String, Vec<String>>,
}

pub(super) fn render(
    rows: &[MatrixRow],
    executable_evidence_ids: &BTreeSet<String>,
) -> anyhow::Result<String> {
    let mut kinds_by_case = BTreeMap::<&str, BTreeSet<&str>>::new();
    let mut automated_cases = BTreeSet::<&str>::new();
    for row in rows.iter().filter(|row| row.case_id != "none") {
        kinds_by_case
            .entry(row.case_id.as_str())
            .or_default()
            .insert(row.evidence_kind.as_str());
        if executable_evidence_ids.contains(&row.evidence_id) {
            automated_cases.insert(row.case_id.as_str());
        }
    }
    let mut document_only_by_family = BTreeMap::<String, Vec<String>>::new();
    let mut unautomated_by_family = BTreeMap::<String, Vec<String>>::new();
    for (case_id, kinds) in kinds_by_case {
        let family = case_id.split('-').next().unwrap_or(case_id).to_owned();
        if kinds.len() == 1 && kinds.contains("document") {
            document_only_by_family
                .entry(family.clone())
                .or_default()
                .push(case_id.to_owned());
        }
        if !automated_cases.contains(case_id) {
            unautomated_by_family
                .entry(family)
                .or_default()
                .push(case_id.to_owned());
        }
    }
    let document_only_total = document_only_by_family.values().map(Vec::len).sum();
    let unautomated_total = unautomated_by_family.values().map(Vec::len).sum();
    serde_json::to_string_pretty(&BacklogReport {
        schema: 1,
        status: "backlog-not-pass-evidence",
        document_only_total,
        unautomated_total,
        document_only_by_family,
        unautomated_by_family,
    })
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(case_id: &str, evidence_kind: &str) -> MatrixRow {
        MatrixRow {
            requirement_id: format!("REQ-{case_id}-{evidence_kind}"),
            journey_id: "none".into(),
            flow_id: "none".into(),
            case_id: case_id.into(),
            surface: "web".into(),
            mode: "browser".into(),
            gate: "ci".into(),
            requirement: "required".into(),
            evidence_kind: evidence_kind.into(),
            evidence_id: format!("{evidence_kind}.{case_id}"),
            evidence_ref: "docs/example.md".into(),
            freshness: "source-bound".into(),
            note: "fixture".into(),
        }
    }

    #[test]
    fn counts_distinct_cases_and_does_not_treat_source_refs_as_automation() {
        let rows = [
            row("UI-001", "document"),
            row("UI-001", "document"),
            row("UI-002", "source-ref"),
            row("AUTH-001", "document"),
            row("AUTH-001", "test"),
            row("FAKE-001", "test"),
        ];
        let json = render(&rows, &BTreeSet::from(["test.AUTH-001".to_owned()])).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["document_only_total"], 1);
        assert_eq!(value["unautomated_total"], 3);
        assert_eq!(value["document_only_by_family"]["UI"][0], "UI-001");
        assert_eq!(value["unautomated_by_family"]["FAKE"][0], "FAKE-001");
    }
}
