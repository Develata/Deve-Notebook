//! Structural validation for acceptance requirements and evidence locators.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use super::model::{FIRST_TAG_JOURNEYS, FlowCase, MatrixRow};
use super::parse::{collect_case_ids, collect_flow_cases};
use super::test_selector::{TestCatalog, validate_test_selector};
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const REQUIREMENTS: [&str; 3] = ["required", "conditional", "non-goal"];
const GATES: [&str; 4] = ["ci", "release", "tag-ready", "advisory"];
const FRESHNESS: [&str; 5] = [
    "source-bound",
    "current-head",
    "target-host-30d",
    "first-tag-once",
    "none",
];
const EVIDENCE_KINDS: [&str; 7] = [
    "source-ref",
    "test",
    "script",
    "document",
    "receipt",
    "external-state",
    "gap",
];

pub(super) fn validate(root: &Path, rows: &[MatrixRow]) -> Result<()> {
    if rows.is_empty() {
        bail!("acceptance-matrix: matrix has no requirement rows");
    }
    let case_ids = collect_case_ids(root)?;
    let flow_cases = collect_flow_cases(root)?;
    let valid_flows: BTreeSet<_> = flow_cases
        .iter()
        .map(|relation| relation.flow_id.clone())
        .collect();
    let test_catalog = rows
        .iter()
        .any(|row| row.evidence_kind == "test")
        .then(|| TestCatalog::load(root))
        .transpose()?;
    let mut requirement_ids = BTreeSet::new();
    let mut mapped_cases = BTreeSet::new();
    let mut mapped_flow_cases = BTreeSet::new();
    let mut journey_contracts = BTreeSet::new();
    let mut evidence_contracts = BTreeMap::<String, (&str, &str)>::new();

    for row in rows {
        require_nonempty(row)?;
        if !requirement_ids.insert(row.requirement_id.clone()) {
            bail!(
                "acceptance-matrix: duplicate requirement_id {}",
                row.requirement_id
            );
        }
        require_enum("requirement", &row.requirement, &REQUIREMENTS, row)?;
        require_enum("gate", &row.gate, &GATES, row)?;
        require_enum("freshness", &row.freshness, &FRESHNESS, row)?;
        require_enum("evidence_kind", &row.evidence_kind, &EVIDENCE_KINDS, row)?;
        if matches!(row.requirement.as_str(), "conditional" | "non-goal")
            && row.note.trim().is_empty()
        {
            bail!(
                "acceptance-matrix: {} requires a concrete rationale",
                row.requirement_id
            );
        }
        if row.requirement == "required" && row.gate == "tag-ready" && row.freshness == "none" {
            bail!(
                "acceptance-matrix: tag-ready requirement {} cannot have freshness=none",
                row.requirement_id
            );
        }
        validate_identity_relations(
            row,
            &case_ids,
            &valid_flows,
            &flow_cases,
            &mut mapped_cases,
            &mut mapped_flow_cases,
            &mut journey_contracts,
        )?;
        validate_evidence(root, row, test_catalog.as_ref())?;
        if let Some((kind, reference)) = evidence_contracts.get(&row.evidence_id) {
            if *kind != row.evidence_kind || *reference != row.evidence_ref {
                bail!(
                    "acceptance-matrix: evidence_id {} has conflicting locators",
                    row.evidence_id
                );
            }
        } else {
            evidence_contracts.insert(
                row.evidence_id.clone(),
                (&row.evidence_kind, &row.evidence_ref),
            );
        }
    }

    let missing_cases: Vec<_> = case_ids.difference(&mapped_cases).cloned().collect();
    if !missing_cases.is_empty() {
        bail!(
            "acceptance-matrix: unbound acceptance cases: {}",
            missing_cases.join(", ")
        );
    }
    let missing_relations: Vec<_> = flow_cases.difference(&mapped_flow_cases).cloned().collect();
    if !missing_relations.is_empty() {
        let summary = missing_relations
            .iter()
            .map(|item| format!("{}:{}", item.flow_id, item.case_id))
            .collect::<Vec<_>>()
            .join(", ");
        bail!("acceptance-matrix: missing operation relations: {summary}");
    }
    for (journey, surface, mode, gate, requirement) in FIRST_TAG_JOURNEYS {
        if !journey_contracts.contains(&(
            journey.to_string(),
            surface.to_string(),
            mode.to_string(),
            gate.to_string(),
            requirement.to_string(),
        )) {
            bail!(
                "acceptance-matrix: missing first-tag contract {journey}:{surface}:{mode}:{gate}:{requirement}"
            );
        }
    }
    Ok(())
}

fn require_nonempty(row: &MatrixRow) -> Result<()> {
    let values = [
        ("requirement_id", row.requirement_id.as_str()),
        ("journey_id", row.journey_id.as_str()),
        ("flow_id", row.flow_id.as_str()),
        ("case_id", row.case_id.as_str()),
        ("surface", row.surface.as_str()),
        ("mode", row.mode.as_str()),
        ("gate", row.gate.as_str()),
        ("requirement", row.requirement.as_str()),
        ("evidence_kind", row.evidence_kind.as_str()),
        ("evidence_id", row.evidence_id.as_str()),
        ("evidence_ref", row.evidence_ref.as_str()),
        ("freshness", row.freshness.as_str()),
    ];
    for (field, value) in values {
        if value.is_empty() {
            bail!(
                "acceptance-matrix: {} has empty {field}",
                row.requirement_id
            );
        }
    }
    Ok(())
}

fn require_enum(field: &str, value: &str, allowed: &[&str], row: &MatrixRow) -> Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        bail!(
            "acceptance-matrix: {} has invalid {field}={value}",
            row.requirement_id
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_identity_relations(
    row: &MatrixRow,
    case_ids: &BTreeSet<String>,
    valid_flows: &BTreeSet<String>,
    flow_cases: &BTreeSet<FlowCase>,
    mapped_cases: &mut BTreeSet<String>,
    mapped_flow_cases: &mut BTreeSet<FlowCase>,
    journey_contracts: &mut BTreeSet<(String, String, String, String, String)>,
) -> Result<()> {
    if row.case_id != "none" {
        if !case_ids.contains(&row.case_id) {
            bail!(
                "acceptance-matrix: {} references unknown case {}",
                row.requirement_id,
                row.case_id
            );
        }
        mapped_cases.insert(row.case_id.clone());
    }
    if row.flow_id != "none" {
        if !valid_flows.contains(&row.flow_id) {
            bail!(
                "acceptance-matrix: {} references unknown flow {}",
                row.requirement_id,
                row.flow_id
            );
        }
        if row.case_id == "none" {
            bail!(
                "acceptance-matrix: {} maps a flow without a case",
                row.requirement_id
            );
        }
        let relation = FlowCase {
            flow_id: row.flow_id.clone(),
            case_id: row.case_id.clone(),
        };
        if !flow_cases.contains(&relation) {
            bail!(
                "acceptance-matrix: {} maps a flow/case relation absent from operation coverage",
                row.requirement_id
            );
        }
        mapped_flow_cases.insert(relation);
    }
    if row.journey_id != "none" {
        let key = (
            row.journey_id.clone(),
            row.surface.clone(),
            row.mode.clone(),
        );
        if !FIRST_TAG_JOURNEYS
            .iter()
            .any(|expected| key.0 == expected.0 && key.1 == expected.1 && key.2 == expected.2)
        {
            bail!(
                "acceptance-matrix: {} references undeclared journey/surface/mode",
                row.requirement_id
            );
        }
        journey_contracts.insert((
            key.0,
            key.1,
            key.2,
            row.gate.clone(),
            row.requirement.clone(),
        ));
    }
    Ok(())
}

fn validate_evidence(
    root: &Path,
    row: &MatrixRow,
    test_catalog: Option<&TestCatalog>,
) -> Result<()> {
    match row.evidence_kind.as_str() {
        "source-ref" | "document" => validate_path(root, &row.evidence_ref, row),
        "script" => {
            let script = row
                .evidence_ref
                .split_whitespace()
                .next()
                .unwrap_or_default();
            if !script.starts_with("scripts/") {
                bail!(
                    "acceptance-matrix: {} script evidence must start with scripts/",
                    row.requirement_id
                );
            }
            validate_path(root, script, row)
        }
        "test" => validate_test_selector(root, row, test_catalog.context("test catalog missing")?),
        "receipt" => validate_receipt_ref(row),
        "gap" => validate_gap(row),
        "external-state" => Ok(()),
        _ => unreachable!(),
    }
}

fn validate_receipt_ref(row: &MatrixRow) -> Result<()> {
    let path = Path::new(&row.evidence_ref);
    if path.is_absolute()
        || path.extension().and_then(|value| value.to_str()) != Some("json")
        || row.evidence_ref.contains('\\')
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::CurDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        bail!(
            "acceptance-matrix: {} receipt locator must be a canonical relative JSON path",
            row.requirement_id
        );
    }
    Ok(())
}

fn validate_gap(row: &MatrixRow) -> Result<()> {
    let note = row.note.trim();
    if note.len() < 16 || note == row.evidence_id || note == row.evidence_ref {
        bail!(
            "acceptance-matrix: {} gap must describe the concrete missing fact",
            row.requirement_id
        );
    }
    Ok(())
}

fn validate_path(root: &Path, reference: &str, row: &MatrixRow) -> Result<()> {
    let rel = reference.split('#').next().unwrap_or_default();
    let path = Path::new(rel);
    if path.is_absolute() || rel.contains("..") || !root.join(path).is_file() {
        bail!(
            "acceptance-matrix: {} evidence path is missing or unsafe: {}",
            row.requirement_id,
            row.evidence_ref
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{require_enum, validate_gap, validate_receipt_ref};
    use crate::acceptance_matrix::model::MatrixRow;

    fn row() -> MatrixRow {
        MatrixRow {
            requirement_id: "req.test".into(),
            journey_id: "none".into(),
            flow_id: "none".into(),
            case_id: "TEST-001".into(),
            surface: "core".into(),
            mode: "unit".into(),
            gate: "ci".into(),
            requirement: "required".into(),
            evidence_kind: "source-ref".into(),
            evidence_id: "source.test".into(),
            evidence_ref: "crates/core/src/lib.rs".into(),
            freshness: "source-bound".into(),
            note: String::new(),
        }
    }

    #[test]
    fn enum_validation_rejects_uncontrolled_values() {
        let row = row();
        assert!(require_enum("gate", "ci", &["ci"], &row).is_ok());
        assert!(require_enum("gate", "maybe", &["ci"], &row).is_err());
    }

    #[test]
    fn gap_and_receipt_locators_are_fail_closed() {
        let mut row = row();
        row.evidence_kind = "gap".into();
        row.note = "missing".into();
        assert!(validate_gap(&row).is_err());
        row.note = "the concrete target-host receipt is missing".into();
        assert!(validate_gap(&row).is_ok());

        row.evidence_ref = "receipts/test.json".into();
        assert!(validate_receipt_ref(&row).is_ok());
        row.evidence_ref = "../outside.json".into();
        assert!(validate_receipt_ref(&row).is_err());
    }
}
