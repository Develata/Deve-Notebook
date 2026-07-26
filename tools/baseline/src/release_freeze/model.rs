//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!   - 18_release#release-versioning
//!
//! Typed first-tag freeze registry. Unknown fields are rejected so governance
//! changes cannot silently bypass the validator.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseFreeze {
    pub schema: u32,
    pub release: ReleaseIdentity,
    pub accepted_gaps: Vec<AcceptedGap>,
    pub artifacts: ArtifactSet,
    pub controls: ControlSet,
    pub excluded: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseIdentity {
    pub version: String,
    pub tag: String,
    pub channel: String,
    pub date: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AcceptedGap {
    pub id: String,
    pub classification: String,
    pub approved_on: String,
    pub bindings: Vec<AcceptedGapBinding>,
    pub release_note_title: String,
    pub user_visible_summary: String,
    pub impact: String,
    pub workaround: String,
    pub exit_condition: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(super) struct AcceptedGapBinding {
    pub requirement_id: String,
    pub evidence_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactSet {
    pub windows_x64_msi: ArtifactPath,
    pub windows_x64_nsis: ArtifactPath,
    pub macos_host_dmg: MacosArtifact,
    pub android_arm64_apk: AndroidArtifact,
    pub docker_linux_amd64_archive: ArtifactPath,
    pub source_spdx: SpdxArtifact,
    pub image_spdx: SpdxArtifact,
    pub provenance_bundle: ArtifactPath,
    pub docker_sbom_bundle: ArtifactPath,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactPath {
    pub path: String,
    pub public: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MacosArtifact {
    pub one_of: Vec<String>,
    pub public: bool,
    pub universal: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AndroidArtifact {
    pub path: String,
    pub public: bool,
    pub signed: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SpdxArtifact {
    pub path: String,
    pub public: bool,
    pub spdx_version: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ControlSet {
    pub release_candidate: ArtifactPath,
    pub candidate_checksums: ArtifactPath,
    pub public_checksums: ArtifactPath,
}
