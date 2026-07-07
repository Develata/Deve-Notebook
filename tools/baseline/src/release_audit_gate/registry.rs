//! plan_ref: infra

use super::LABEL;
use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const REGISTRY_REL: &str = "docs/registry/release-audit-warning-registry.md";
const VALID_DECISIONS: &[&str] = &[
    "direct-migration-before-stable",
    "feature-gated-upstream-watch",
    "upstream-upgrade-watch",
];

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AuditWarning {
    advisory: String,
    krate: String,
    version: String,
    kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RegistryRow {
    warning: AuditWarning,
    version: String,
    decision: String,
    tag_blocker: String,
    rationale: String,
    route: String,
}

pub(super) fn validate_registry_file(root: &Path) -> Result<()> {
    let registry = read_registry(root)?;
    parse_registry(&registry)?;
    Ok(())
}

pub(super) fn read_registry(root: &Path) -> Result<String> {
    fs::read_to_string(root.join(REGISTRY_REL))
        .map_err(|err| anyhow::anyhow!("{LABEL}: failed to read {REGISTRY_REL}: {err}"))
}

pub(super) fn validate_cargo_audit_report(report: &str, registry: &str) -> Result<()> {
    let actual = parse_audit_warnings(report)?;
    let rows = parse_registry(registry)?;
    let expected: BTreeSet<AuditWarning> = rows.into_iter().map(|row| row.warning).collect();

    let missing: Vec<_> = actual.difference(&expected).cloned().collect();
    let stale: Vec<_> = expected.difference(&actual).cloned().collect();
    if !missing.is_empty() || !stale.is_empty() {
        bail!(
            "{LABEL}: cargo audit warnings do not match {REGISTRY_REL}; missing registry rows: {}; stale registry rows: {}",
            format_warnings(&missing),
            format_warnings(&stale)
        );
    }
    Ok(())
}

pub(super) fn validate_no_tag_blockers(registry: &str) -> Result<()> {
    let blockers: Vec<_> = parse_registry(registry)?
        .into_iter()
        .filter(|row| row.tag_blocker == "yes")
        .map(|row| row.warning)
        .collect();
    if !blockers.is_empty() {
        bail!(
            "{LABEL}: first-tag readiness is blocked by registered audit warnings: {}",
            format_warnings(&blockers)
        );
    }
    Ok(())
}

fn parse_registry(registry: &str) -> Result<Vec<RegistryRow>> {
    let mut rows = Vec::new();
    let mut seen = BTreeMap::<AuditWarning, usize>::new();
    for (line_no, line) in registry.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("| RUSTSEC-") {
            continue;
        }
        let fields = split_markdown_row(trimmed);
        if fields.len() != 8 {
            bail!(
                "{LABEL}: {REGISTRY_REL}:{} expected 8 table cells, got {}",
                line_no + 1,
                fields.len()
            );
        }
        let row = RegistryRow {
            warning: AuditWarning {
                advisory: fields[0].clone(),
                krate: fields[1].clone(),
                version: fields[2].clone(),
                kind: fields[3].clone(),
            },
            version: fields[2].clone(),
            decision: fields[4].clone(),
            tag_blocker: fields[5].clone(),
            rationale: fields[6].clone(),
            route: fields[7].clone(),
        };
        validate_registry_row(line_no + 1, &row)?;
        if let Some(previous) = seen.insert(row.warning.clone(), line_no + 1) {
            bail!(
                "{LABEL}: {REGISTRY_REL}:{} duplicates audit warning already listed at line {}",
                line_no + 1,
                previous
            );
        }
        rows.push(row);
    }
    if rows.is_empty() {
        bail!("{LABEL}: {REGISTRY_REL} has no RUSTSEC warning rows");
    }
    Ok(rows)
}

fn validate_registry_row(line_no: usize, row: &RegistryRow) -> Result<()> {
    if !row.warning.advisory.starts_with("RUSTSEC-") {
        bail!("{LABEL}: {REGISTRY_REL}:{line_no} advisory must start with RUSTSEC-");
    }
    for (label, value) in [
        ("crate", &row.warning.krate),
        ("version", &row.version),
        ("kind", &row.warning.kind),
        ("decision", &row.decision),
        ("tag blocker", &row.tag_blocker),
        ("rationale", &row.rationale),
        ("replacement route", &row.route),
    ] {
        if value.trim().is_empty() || matches!(value.trim(), "TODO" | "TBD" | "todo" | "tbd") {
            bail!("{LABEL}: {REGISTRY_REL}:{line_no} {label} must be concrete");
        }
    }
    if !matches!(
        row.warning.kind.as_str(),
        "unmaintained" | "unsound" | "notice"
    ) {
        bail!(
            "{LABEL}: {REGISTRY_REL}:{line_no} unsupported warning kind '{}'",
            row.warning.kind
        );
    }
    if !VALID_DECISIONS.contains(&row.decision.as_str()) {
        bail!(
            "{LABEL}: {REGISTRY_REL}:{line_no} unsupported decision '{}'",
            row.decision
        );
    }
    if !matches!(row.tag_blocker.as_str(), "yes" | "no") {
        bail!("{LABEL}: {REGISTRY_REL}:{line_no} tag blocker must be yes or no");
    }
    Ok(())
}

fn split_markdown_row(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().trim_matches('`').to_string())
        .collect()
}

fn parse_audit_warnings(report: &str) -> Result<BTreeSet<AuditWarning>> {
    let json: Value = serde_json::from_str(report)?;
    validate_no_vulnerabilities(&json)?;
    let mut warnings = BTreeSet::new();
    let Some(warning_groups) = json.get("warnings").and_then(Value::as_object) else {
        return Ok(warnings);
    };
    for (kind, entries) in warning_groups {
        let Some(entries) = entries.as_array() else {
            bail!("{LABEL}: cargo audit JSON warning group '{kind}' is not an array");
        };
        for entry in entries {
            let advisory = entry
                .get("advisory")
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("{LABEL}: cargo audit warning missing advisory.id")
                })?;
            let krate = entry
                .get("package")
                .and_then(|value| value.get("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("{LABEL}: cargo audit warning missing package.name")
                })?;
            let version = entry
                .get("package")
                .and_then(|value| value.get("version"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("{LABEL}: cargo audit warning missing package.version")
                })?;
            warnings.insert(AuditWarning {
                advisory: advisory.to_string(),
                krate: krate.to_string(),
                version: version.to_string(),
                kind: kind.to_string(),
            });
        }
    }
    Ok(warnings)
}

fn validate_no_vulnerabilities(json: &Value) -> Result<()> {
    let vulnerabilities = json
        .get("vulnerabilities")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            anyhow::anyhow!("{LABEL}: cargo audit JSON missing vulnerabilities object")
        })?;
    let found = vulnerabilities
        .get("found")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            anyhow::anyhow!("{LABEL}: cargo audit JSON missing vulnerabilities.found")
        })?;
    let count = vulnerabilities
        .get("count")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            anyhow::anyhow!("{LABEL}: cargo audit JSON missing vulnerabilities.count")
        })?;
    if found || count > 0 {
        bail!("{LABEL}: cargo audit reported {count} vulnerabilities");
    }
    Ok(())
}

fn format_warnings(warnings: &[AuditWarning]) -> String {
    if warnings.is_empty() {
        return "none".to_string();
    }
    warnings
        .iter()
        .map(|warning| {
            format!(
                "{}:{}@{}:{}",
                warning.advisory, warning.krate, warning.version, warning.kind
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::{
        AuditWarning, parse_audit_warnings, parse_registry, validate_cargo_audit_report,
        validate_no_tag_blockers,
    };

    #[test]
    fn parses_registry_rows_with_concrete_routes() {
        let rows = parse_registry(
            "| Advisory | Crate | Version | Kind | Decision | Tag blocker | Rationale | Replacement route |\n\
             |---|---|---|---|---|---|---|---|\n\
             | RUSTSEC-0000-0001 | demo | 1.0.0 | unmaintained | upstream-upgrade-watch | no | transitive framework dependency | track upstream release |\n",
        )
        .expect("registry");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].warning.krate, "demo");
    }

    #[test]
    fn rejects_registry_rows_without_routes() {
        let error = parse_registry(
            "| RUSTSEC-0000-0001 | demo | 1.0.0 | unmaintained | upstream-upgrade-watch | no | rationale | TODO |\n",
        )
        .expect_err("invalid registry");

        assert!(error.to_string().contains("replacement route"));
    }

    #[test]
    fn parses_cargo_audit_warning_report() {
        let warnings = parse_audit_warnings(
            r#"{
              "vulnerabilities": {"found": false, "count": 0},
              "warnings": {
                "unmaintained": [
                  {"advisory": {"id": "RUSTSEC-0000-0001"}, "package": {"name": "demo", "version": "1.0.0"}}
                ],
                "unsound": []
              }
            }"#,
        )
        .expect("warnings");

        assert!(warnings.contains(&AuditWarning {
            advisory: "RUSTSEC-0000-0001".to_string(),
            krate: "demo".to_string(),
            version: "1.0.0".to_string(),
            kind: "unmaintained".to_string(),
        }));
    }

    #[test]
    fn compares_cargo_audit_report_to_registry() {
        let report = r#"{
          "vulnerabilities": {"found": false, "count": 0},
          "warnings": {
            "unmaintained": [
              {"advisory": {"id": "RUSTSEC-0000-0001"}, "package": {"name": "demo", "version": "1.0.0"}}
            ]
          }
        }"#;
        let registry = "| RUSTSEC-0000-0001 | demo | 1.0.0 | unmaintained | upstream-upgrade-watch | no | reason | route |\n";

        validate_cargo_audit_report(report, registry).expect("matching registry");
    }

    #[test]
    fn rejects_unregistered_cargo_audit_warning() {
        let report = r#"{
          "vulnerabilities": {"found": false, "count": 0},
          "warnings": {
            "unmaintained": [
              {"advisory": {"id": "RUSTSEC-0000-0002"}, "package": {"name": "new", "version": "1.0.0"}}
            ]
          }
        }"#;
        let registry = "| RUSTSEC-0000-0001 | demo | 1.0.0 | unmaintained | upstream-upgrade-watch | no | reason | route |\n";

        let error =
            validate_cargo_audit_report(report, registry).expect_err("unregistered warning");
        assert!(error.to_string().contains("missing registry rows"));
    }

    #[test]
    fn tag_ready_rejects_registered_blockers() {
        let registry = "| RUSTSEC-0000-0001 | demo | 1.0.0 | unmaintained | upstream-upgrade-watch | yes | reason | route |\n";

        let error = validate_no_tag_blockers(registry).expect_err("tag blocker");
        assert!(error.to_string().contains("first-tag readiness is blocked"));
    }

    #[test]
    fn rejects_cargo_audit_vulnerabilities_even_when_report_is_readable() {
        let report = r#"{
          "vulnerabilities": {"found": true, "count": 1},
          "warnings": {}
        }"#;
        let registry = "| RUSTSEC-0000-0001 | demo | 1.0.0 | unmaintained | upstream-upgrade-watch | no | reason | route |\n";

        let error =
            validate_cargo_audit_report(report, registry).expect_err("vulnerability report");
        assert!(error.to_string().contains("reported 1 vulnerabilities"));
    }

    #[test]
    fn rejects_cargo_audit_reports_without_vulnerability_summary() {
        let report = r#"{"warnings": {}}"#;
        let registry = "| RUSTSEC-0000-0001 | demo | 1.0.0 | unmaintained | upstream-upgrade-watch | no | reason | route |\n";

        let error = validate_cargo_audit_report(report, registry).expect_err("missing summary");
        assert!(error.to_string().contains("missing vulnerabilities object"));
    }
}
