//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!
//! Canonical schema owned by the release-candidate baseline command.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Take};
use std::path::Path;

const MAX_SPDX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ATTESTATION_BUNDLE_BYTES: u64 = 16 * 1024 * 1024;

pub(super) const MANIFEST_SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ArtifactRole {
    WindowsMsi,
    WindowsNsis,
    MacosDmg,
    AndroidArm64Apk,
    DockerLinuxAmd64Archive,
    SourceSpdx,
    ImageSpdx,
    ProvenanceAttestation,
    DockerSbomAttestation,
}

impl ArtifactRole {
    pub(super) fn is_public(self) -> bool {
        !matches!(self, Self::DockerLinuxAmd64Archive)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactRecord {
    pub role: ArtifactRole,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub public: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkflowIdentity {
    pub path: String,
    pub run_id: u64,
    pub run_attempt: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChecksumFiles {
    pub public: String,
    pub internal: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CandidateManifest {
    pub schema: u32,
    pub head_sha: String,
    pub version: String,
    pub workflow: WorkflowIdentity,
    pub docker_image_id: String,
    pub android_signer_sha256: String,
    pub artifacts: Vec<ArtifactRecord>,
    pub checksums: ChecksumFiles,
}

pub(super) fn canonical_json(manifest: &CandidateManifest) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(manifest).context("serialize release candidate")?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn read_manifest(bytes: &[u8]) -> Result<CandidateManifest> {
    serde_json::from_slice(bytes).context("parse release candidate manifest")
}

pub(super) fn checksum_lines(entries: &BTreeMap<String, String>) -> String {
    let mut result = String::new();
    for (path, hash) in entries {
        result.push_str(hash);
        result.push_str("  ");
        result.push_str(path);
        result.push('\n');
    }
    result
}

pub(super) fn validate_spdx(path: &Path, role: ArtifactRole) -> Result<()> {
    let value = read_json(path, "SPDX document", MAX_SPDX_BYTES)?;
    let document = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{} is not an SPDX JSON object", path.display()))?;
    match document.get("spdxVersion").and_then(Value::as_str) {
        Some("SPDX-2.3") => {}
        Some(other) => bail!(
            "{} declares unsupported SPDX version {other}; expected SPDX-2.3",
            path.display()
        ),
        None => bail!(
            "{} does not declare string field spdxVersion",
            path.display()
        ),
    }

    require_exact_string(document, "dataLicense", "CC0-1.0", path)?;
    require_exact_string(document, "SPDXID", "SPDXRef-DOCUMENT", path)?;
    require_non_empty_string(document, "name", path)?;
    require_non_empty_string(document, "documentNamespace", path)?;
    let creation = document
        .get("creationInfo")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("{} is missing object creationInfo", path.display()))?;
    require_non_empty_string(creation, "created", path)?;
    let creators = creation
        .get("creators")
        .and_then(Value::as_array)
        .filter(|creators| {
            !creators.is_empty()
                && creators
                    .iter()
                    .all(|creator| creator.as_str().is_some_and(|value| !value.is_empty()))
        })
        .ok_or_else(|| anyhow::anyhow!("{} has no valid SPDX creators", path.display()))?;
    debug_assert!(!creators.is_empty());

    let packages = valid_object_array(document.get("packages"), &["SPDXID", "name"]);
    let files = valid_object_array(document.get("files"), &["SPDXID", "fileName"]);
    let relationships = valid_object_array(
        document.get("relationships"),
        &["spdxElementId", "relationshipType", "relatedSpdxElement"],
    );
    match role {
        ArtifactRole::SourceSpdx if !packages && !files => bail!(
            "{} source SPDX document describes no packages or files",
            path.display()
        ),
        ArtifactRole::ImageSpdx if !packages => bail!(
            "{} image SPDX document describes no packages",
            path.display()
        ),
        ArtifactRole::SourceSpdx | ArtifactRole::ImageSpdx if !relationships => {
            bail!("{} SPDX document has no relationships", path.display())
        }
        ArtifactRole::SourceSpdx | ArtifactRole::ImageSpdx => {}
        _ => bail!("internal error: SPDX validation requested for non-SPDX artifact"),
    }
    Ok(())
}

pub(super) fn validate_attestation_bundle(path: &Path) -> Result<()> {
    let bytes = read_bounded(path, "attestation bundle", MAX_ATTESTATION_BUNDLE_BYTES)?;
    if bytes.is_empty() {
        bail!("attestation bundle is empty: {}", path.display());
    }
    if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
        if value.is_object() {
            return Ok(());
        }
        bail!(
            "attestation bundle must contain a JSON object: {}",
            path.display()
        );
    }

    let text = std::str::from_utf8(&bytes)
        .with_context(|| format!("attestation bundle {} is not UTF-8", path.display()))?;
    let mut records = 0usize;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str::<Value>(line).with_context(|| {
            format!(
                "parse attestation bundle {} JSONL record {}",
                path.display(),
                index + 1
            )
        })?;
        if !value.is_object() {
            bail!(
                "attestation bundle {} JSONL record {} is not an object",
                path.display(),
                index + 1
            );
        }
        records += 1;
    }
    if records == 0 {
        bail!(
            "attestation bundle contains no JSON records: {}",
            path.display()
        );
    }
    Ok(())
}

fn require_non_empty_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
    path: &Path,
) -> Result<()> {
    if object
        .get(field)
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        bail!("{} is missing non-empty SPDX field {field}", path.display());
    }
    Ok(())
}

fn require_exact_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
    expected: &str,
    path: &Path,
) -> Result<()> {
    if object.get(field).and_then(Value::as_str) != Some(expected) {
        bail!(
            "{} SPDX field {field} must be exactly {expected}",
            path.display()
        );
    }
    Ok(())
}

fn valid_object_array(value: Option<&Value>, fields: &[&str]) -> bool {
    value.and_then(Value::as_array).is_some_and(|values| {
        !values.is_empty()
            && values.iter().all(|value| {
                value.as_object().is_some_and(|object| {
                    fields.iter().all(|field| {
                        object
                            .get(*field)
                            .and_then(Value::as_str)
                            .is_some_and(|value| !value.is_empty())
                    })
                })
            })
    })
}

fn read_json(path: &Path, label: &str, maximum: u64) -> Result<Value> {
    let bytes = read_bounded(path, label, maximum)?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parse {label} {} as JSON", path.display()))
}

fn read_bounded(path: &Path, label: &str, maximum: u64) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("open {label} {}", path.display()))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("inspect {label} {}", path.display()))?;
    if metadata.len() > maximum {
        bail!(
            "{label} {} exceeds the {maximum}-byte resource limit",
            path.display()
        );
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut bounded: Take<File> = file.take(maximum + 1);
    bounded
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {label} {}", path.display()))?;
    if bytes.len() as u64 > maximum {
        bail!(
            "{label} {} grew beyond the {maximum}-byte resource limit",
            path.display()
        );
    }
    Ok(bytes)
}
