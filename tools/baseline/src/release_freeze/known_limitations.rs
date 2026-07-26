//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix
//!   - 18_release#release-versioning
//!
//! Exact-version accepted-gap and public release-note projection. The gap
//! remains visible in the acceptance matrix; this module only admits the one
//! USER-approved Public Preview limitation and never converts it into evidence.

use super::model::{AcceptedGap, AcceptedGapBinding, ReleaseFreeze};
use anyhow::{Result, bail, ensure};
use std::collections::BTreeMap;

pub(crate) type AcceptedGapBindings = BTreeMap<(String, String), String>;

const ACCEPTED_GAP_ID: &str = "known-limitation.windows-watcher-overflow";
const ACCEPTED_GAP_CLASSIFICATION: &str = "public-preview-known-limitation";
const ACCEPTED_REQUIREMENT_ID: &str = "case.store-016";
const ACCEPTED_EVIDENCE_ID: &str = "gap.watcher.windows-overflow-receipt";

pub(super) fn validate(registry: &ReleaseFreeze, changelog: &str) -> Result<AcceptedGapBindings> {
    ensure!(
        registry.accepted_gaps.len() == 1,
        "v0.1.0 Public Preview must contain exactly one accepted gap"
    );
    let gap = &registry.accepted_gaps[0];
    ensure!(
        gap.id == ACCEPTED_GAP_ID
            && gap.classification == ACCEPTED_GAP_CLASSIFICATION
            && gap.approved_on == registry.release.date,
        "accepted gap identity, classification, or approval date is not the approved v0.1.0 contract"
    );
    ensure!(
        gap.bindings
            == [AcceptedGapBinding {
                requirement_id: ACCEPTED_REQUIREMENT_ID.to_owned(),
                evidence_id: ACCEPTED_EVIDENCE_ID.to_owned(),
            }],
        "only the STORE-016 Windows overflow gap may be accepted for v0.1.0"
    );
    for (label, value) in [
        ("release_note_title", gap.release_note_title.as_str()),
        ("user_visible_summary", gap.user_visible_summary.as_str()),
        ("impact", gap.impact.as_str()),
        ("workaround", gap.workaround.as_str()),
        ("exit_condition", gap.exit_condition.as_str()),
    ] {
        ensure!(
            !value.trim().is_empty()
                && !value.contains('\n')
                && !value.contains('\r')
                && !value.chars().any(char::is_control)
                && !value.contains("大量"),
            "accepted gap {label} must be one non-empty printable line"
        );
    }
    ensure!(
        gap.user_visible_summary.contains("数千"),
        "Windows watcher release note must quantify the burst as 数千, not 大量"
    );

    let release_notes = release_notes_section(registry, changelog)?;
    let limitation = limitation_markdown(gap);
    let heading = "### Known limitations";
    ensure!(
        release_notes.matches(heading).count() == 1,
        "CHANGELOG release section must contain exactly one Known limitations heading"
    );
    let limitation_start = release_notes
        .find(heading)
        .expect("validated Known limitations heading");
    ensure!(
        release_notes[limitation_start..].trim_end() == limitation,
        "CHANGELOG Known limitations section must exactly equal the accepted-gap projection"
    );

    let mut bindings = AcceptedGapBindings::new();
    for binding in &gap.bindings {
        let key = (binding.requirement_id.clone(), binding.evidence_id.clone());
        ensure!(
            bindings.insert(key, gap.id.clone()).is_none(),
            "accepted gap contains a duplicate requirement/evidence binding"
        );
    }
    Ok(bindings)
}

pub(super) fn release_notes(registry: &ReleaseFreeze, changelog: &str) -> Result<String> {
    validate(registry, changelog)?;
    release_notes_section(registry, changelog)
}

pub(super) fn validate_candidate(registry: &ReleaseFreeze, changelog: &str) -> Result<()> {
    validate(registry, changelog)?;
    let release_heading = format!(
        "## [{}] - {}",
        registry.release.version, registry.release.date
    );
    let release_start = changelog
        .find(&release_heading)
        .expect("validated frozen release heading");
    let unreleased = "## [Unreleased]";
    ensure!(
        changelog.matches(unreleased).count() == 1,
        "CHANGELOG must contain exactly one [Unreleased] heading"
    );
    let unreleased_start = changelog
        .find(unreleased)
        .expect("validated Unreleased heading");
    ensure!(
        unreleased_start < release_start,
        "CHANGELOG [Unreleased] must precede the frozen release"
    );
    ensure!(
        changelog[unreleased_start + unreleased.len()..release_start]
            .trim()
            .is_empty(),
        "CHANGELOG [Unreleased] must be empty at the frozen candidate HEAD"
    );
    Ok(())
}

fn release_notes_section(registry: &ReleaseFreeze, changelog: &str) -> Result<String> {
    let heading = format!(
        "## [{}] - {}",
        registry.release.version, registry.release.date
    );
    ensure!(
        changelog.matches(&heading).count() == 1,
        "CHANGELOG must contain exactly one frozen release heading {heading}"
    );
    let start = changelog
        .find(&heading)
        .expect("validated frozen release heading");
    let after_heading = start + heading.len();
    let end = changelog[after_heading..]
        .find("\n## [")
        .map(|offset| after_heading + offset)
        .unwrap_or(changelog.len());
    let section = changelog[start..end].trim_end();
    ensure!(
        !section.is_empty(),
        "frozen CHANGELOG release section must not be empty"
    );

    Ok(format!("{section}\n"))
}

fn limitation_markdown(gap: &AcceptedGap) -> String {
    format!(
        "### Known limitations\n- **{}**：{}\n  - 影响：{}\n  - 规避：{}\n  - 退出条件：{}",
        gap.release_note_title,
        gap.user_visible_summary,
        gap.impact,
        gap.workaround,
        gap.exit_condition
    )
}

pub(crate) fn reject_unconsumed(bindings: &AcceptedGapBindings) -> Result<()> {
    if bindings.is_empty() {
        Ok(())
    } else {
        let entries = bindings
            .iter()
            .map(|((requirement, evidence), id)| format!("{id}:{requirement}/{evidence}"))
            .collect::<Vec<_>>()
            .join(", ");
        bail!("accepted gaps do not match current required tag-ready gaps: {entries}")
    }
}
