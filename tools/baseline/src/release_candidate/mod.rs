//! plan_ref: docs/plan/18_release.md#artifact-identity-and-integrity
//!
//! Deterministic, fail-closed assembly and verification of a pre-tag release
//! candidate.  The workflow owns platform-specific builds and signing; this
//! module owns the portable artifact allowlist, path containment, identities,
//! hashes, and canonical manifest/checksum projections.

mod args;
mod manifest;
mod paths;

use anyhow::{Result, bail};
use args::{Action, CandidateArgs};
use manifest::{
    ArtifactRecord, CandidateManifest, ChecksumFiles, MANIFEST_SCHEMA, WorkflowIdentity,
    canonical_json, checksum_lines, read_manifest,
};
use paths::{CandidateRoot, ResolvedArtifact};
use std::collections::{BTreeMap, BTreeSet};

const PUBLIC_CHECKSUMS: &str = "SHA256SUMS";
const INTERNAL_CHECKSUMS: &str = "release-candidate-SHA256SUMS";
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_CHECKSUM_BYTES: u64 = 256 * 1024;

pub fn run(args: &[String]) -> Result<()> {
    let (action, args) = CandidateArgs::parse(args)?;
    match action {
        Action::Assemble => assemble(&args),
        Action::Verify => verify(&args),
    }
}

fn assemble(args: &CandidateArgs) -> Result<()> {
    let root = CandidateRoot::open(&args.candidate_dir)?;
    let artifacts = resolve_artifacts(&root, args)?;
    root.validate_inventory(&artifact_inventory(&artifacts))?;
    validate_structured_inputs(&artifacts)?;

    let manifest = CandidateManifest {
        schema: MANIFEST_SCHEMA,
        head_sha: args.head_sha.clone(),
        version: args.version.clone(),
        workflow: WorkflowIdentity {
            path: args.workflow_path.clone(),
            run_id: args.run_id,
            run_attempt: args.run_attempt,
        },
        docker_image_id: args.docker_image_id.clone(),
        android_signer_sha256: args.android_signer_sha256.clone(),
        artifacts: artifacts.iter().map(ResolvedArtifact::record).collect(),
        checksums: ChecksumFiles {
            public: PUBLIC_CHECKSUMS.to_owned(),
            internal: INTERNAL_CHECKSUMS.to_owned(),
        },
    };

    let manifest_bytes = canonical_json(&manifest)?;
    let mut public = public_records_by_basename(&manifest.artifacts)?;
    public.insert(args.output.clone(), paths::sha256_bytes(&manifest_bytes));
    let public_bytes = checksum_lines(&public).into_bytes();
    let mut internal = records_by_path(&manifest.artifacts, false);
    internal.insert(args.output.clone(), paths::sha256_bytes(&manifest_bytes));
    internal.insert(
        PUBLIC_CHECKSUMS.to_owned(),
        paths::sha256_bytes(&public_bytes),
    );
    let internal_bytes = checksum_lines(&internal).into_bytes();

    let mut generated = Vec::with_capacity(3);
    let result = (|| {
        root.write_generated(&args.output, &manifest_bytes)?;
        generated.push(args.output.as_str());
        root.write_generated(PUBLIC_CHECKSUMS, &public_bytes)?;
        generated.push(PUBLIC_CHECKSUMS);
        root.write_generated(INTERNAL_CHECKSUMS, &internal_bytes)?;
        generated.push(INTERNAL_CHECKSUMS);
        verify(args)
    })();
    if result.is_err() {
        root.remove_generated(&generated)?;
    }
    result
}

fn verify(args: &CandidateArgs) -> Result<()> {
    let root = CandidateRoot::open(&args.candidate_dir)?;
    let artifacts = resolve_artifacts(&root, args)?;
    root.validate_inventory(&expected_inventory(args, &artifacts))?;
    validate_structured_inputs(&artifacts)?;

    let manifest_bytes = root.read_bounded_control(&args.output, MAX_MANIFEST_BYTES)?;
    let manifest = read_manifest(&manifest_bytes)?;
    validate_manifest_identity(&manifest, args)?;

    let canonical = canonical_json(&manifest)?;
    if canonical != manifest_bytes {
        bail!("release candidate manifest is not canonical JSON");
    }

    let expected_records: Vec<_> = artifacts.iter().map(ResolvedArtifact::record).collect();
    if manifest.artifacts != expected_records {
        bail!("release candidate artifact records do not match the exact input allowlist");
    }

    let mut public = public_records_by_basename(&manifest.artifacts)?;
    public.insert(args.output.clone(), paths::sha256_bytes(&manifest_bytes));
    root.require_exact_file(PUBLIC_CHECKSUMS, checksum_lines(&public).as_bytes())?;

    let public_bytes = root.read_bounded_control(PUBLIC_CHECKSUMS, MAX_CHECKSUM_BYTES)?;
    let mut internal = records_by_path(&manifest.artifacts, false);
    internal.insert(args.output.clone(), paths::sha256_bytes(&manifest_bytes));
    internal.insert(
        PUBLIC_CHECKSUMS.to_owned(),
        paths::sha256_bytes(&public_bytes),
    );
    root.require_exact_file(INTERNAL_CHECKSUMS, checksum_lines(&internal).as_bytes())?;

    println!(
        "release-candidate: verified schema {} version {} head {} ({} artifacts)",
        manifest.schema,
        manifest.version,
        manifest.head_sha,
        manifest.artifacts.len()
    );
    Ok(())
}

fn resolve_artifacts(root: &CandidateRoot, args: &CandidateArgs) -> Result<Vec<ResolvedArtifact>> {
    let mut resolved = Vec::with_capacity(args.artifacts.len());
    for input in &args.artifacts {
        resolved.push(root.resolve_artifact(input.role, &input.path)?);
    }
    resolved.sort_by(|left, right| {
        left.role
            .cmp(&right.role)
            .then_with(|| left.relative.cmp(&right.relative))
    });

    let unique_paths: BTreeSet<_> = resolved
        .iter()
        .map(|artifact| artifact.relative.as_str())
        .collect();
    if unique_paths.len() != resolved.len() {
        bail!("one file cannot satisfy multiple release candidate artifact roles");
    }
    Ok(resolved)
}

fn expected_inventory(args: &CandidateArgs, artifacts: &[ResolvedArtifact]) -> BTreeSet<String> {
    let mut expected = artifact_inventory(artifacts);
    expected.extend(generated_inventory(args));
    expected
}

fn artifact_inventory(artifacts: &[ResolvedArtifact]) -> BTreeSet<String> {
    artifacts
        .iter()
        .map(|artifact| artifact.relative.clone())
        .collect()
}

fn generated_inventory(args: &CandidateArgs) -> BTreeSet<String> {
    BTreeSet::from([
        args.output.clone(),
        PUBLIC_CHECKSUMS.to_owned(),
        INTERNAL_CHECKSUMS.to_owned(),
    ])
}

fn records_by_path(records: &[ArtifactRecord], public_only: bool) -> BTreeMap<String, String> {
    records
        .iter()
        .filter(|record| !public_only || record.public)
        .map(|record| (record.path.clone(), record.sha256.clone()))
        .collect()
}

fn public_records_by_basename(records: &[ArtifactRecord]) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for record in records.iter().filter(|record| record.public) {
        let name = record.path.rsplit('/').next().unwrap_or_default();
        if name.is_empty()
            || result
                .insert(name.to_owned(), record.sha256.clone())
                .is_some()
        {
            bail!("public release artifacts must have unique non-empty basenames");
        }
    }
    Ok(result)
}

fn validate_manifest_identity(manifest: &CandidateManifest, args: &CandidateArgs) -> Result<()> {
    if manifest.schema != MANIFEST_SCHEMA {
        bail!(
            "unsupported release candidate schema {}; expected {}",
            manifest.schema,
            MANIFEST_SCHEMA
        );
    }
    if manifest.head_sha != args.head_sha {
        bail!("release candidate HEAD does not match the requested HEAD");
    }
    if manifest.version != args.version {
        bail!("release candidate version does not match the requested version");
    }
    if manifest.workflow.path != args.workflow_path
        || manifest.workflow.run_id != args.run_id
        || manifest.workflow.run_attempt != args.run_attempt
    {
        bail!("release candidate workflow identity does not match the requested run");
    }
    if manifest.docker_image_id != args.docker_image_id {
        bail!("release candidate Docker image ID does not match");
    }
    if manifest.android_signer_sha256 != args.android_signer_sha256 {
        bail!("release candidate Android signer certificate does not match");
    }
    if manifest.checksums.public != PUBLIC_CHECKSUMS
        || manifest.checksums.internal != INTERNAL_CHECKSUMS
    {
        bail!("release candidate checksum file identities are invalid");
    }
    Ok(())
}

fn validate_structured_inputs(artifacts: &[ResolvedArtifact]) -> Result<()> {
    for artifact in artifacts {
        match artifact.role {
            manifest::ArtifactRole::SourceSpdx | manifest::ArtifactRole::ImageSpdx => {
                manifest::validate_spdx(&artifact.absolute, artifact.role)?;
            }
            manifest::ArtifactRole::ProvenanceAttestation
            | manifest::ArtifactRole::DockerSbomAttestation => {
                manifest::validate_attestation_bundle(&artifact.absolute)?;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
