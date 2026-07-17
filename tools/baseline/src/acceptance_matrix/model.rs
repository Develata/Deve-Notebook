//! Acceptance matrix data model.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

pub(super) const MATRIX_PATH: &str = "docs/registry/acceptance-matrix.tsv";
pub(super) const RENDERED_PATH: &str = "docs/acceptance-matrix.md";
pub(super) const HEADER: [&str; 13] = [
    "requirement_id",
    "journey_id",
    "flow_id",
    "case_id",
    "surface",
    "mode",
    "gate",
    "requirement",
    "evidence_kind",
    "evidence_id",
    "evidence_ref",
    "freshness",
    "note",
];

pub(super) const FIRST_TAG_JOURNEYS: [(&str, &str, &str, &str, &str); 17] = [
    ("auth-session", "web", "browser", "tag-ready", "required"),
    ("repo-lifecycle", "web", "browser", "tag-ready", "required"),
    (
        "edit-sync-offline-recovery",
        "docker",
        "multiclient",
        "tag-ready",
        "required",
    ),
    ("source-control", "web", "browser", "tag-ready", "required"),
    (
        "external-changes",
        "web",
        "browser",
        "tag-ready",
        "required",
    ),
    ("remote-import", "web", "browser", "tag-ready", "required"),
    (
        "notegit",
        "desktop",
        "local-backend",
        "tag-ready",
        "required",
    ),
    (
        "p2p-gap-recovery",
        "docker",
        "mesh",
        "tag-ready",
        "required",
    ),
    (
        "docker-multiclient",
        "docker",
        "browser",
        "tag-ready",
        "required",
    ),
    (
        "desktop-local-backend",
        "desktop",
        "local-backend",
        "tag-ready",
        "required",
    ),
    (
        "desktop-remote-browser",
        "desktop",
        "remote-browser",
        "tag-ready",
        "required",
    ),
    (
        "android-local-backend",
        "android",
        "local-backend",
        "tag-ready",
        "required",
    ),
    (
        "android-remote-browser",
        "android",
        "remote-browser",
        "tag-ready",
        "required",
    ),
    (
        "release-artifacts",
        "release",
        "multi-platform",
        "tag-ready",
        "required",
    ),
    (
        "release-artifacts",
        "macos",
        "target-host",
        "advisory",
        "conditional",
    ),
    (
        "release-artifacts",
        "ios",
        "target-host",
        "advisory",
        "conditional",
    ),
    (
        "security-supply-chain",
        "github",
        "repository",
        "tag-ready",
        "required",
    ),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MatrixRow {
    pub requirement_id: String,
    pub journey_id: String,
    pub flow_id: String,
    pub case_id: String,
    pub surface: String,
    pub mode: String,
    pub gate: String,
    pub requirement: String,
    pub evidence_kind: String,
    pub evidence_id: String,
    pub evidence_ref: String,
    pub freshness: String,
    pub note: String,
}

impl MatrixRow {
    pub(super) fn from_fields(fields: &[&str]) -> Self {
        Self {
            requirement_id: fields[0].to_string(),
            journey_id: fields[1].to_string(),
            flow_id: fields[2].to_string(),
            case_id: fields[3].to_string(),
            surface: fields[4].to_string(),
            mode: fields[5].to_string(),
            gate: fields[6].to_string(),
            requirement: fields[7].to_string(),
            evidence_kind: fields[8].to_string(),
            evidence_id: fields[9].to_string(),
            evidence_ref: fields[10].to_string(),
            freshness: fields[11].to_string(),
            note: fields[12].to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FlowCase {
    pub flow_id: String,
    pub case_id: String,
}
