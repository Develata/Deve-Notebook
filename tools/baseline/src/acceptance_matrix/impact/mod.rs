//! Shadow-only dependency-aware CI impact planning.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

mod backlog;
mod model;
mod plan;
mod registry;
mod shadow;

use crate::acceptance_matrix::model::MATRIX_PATH;
use crate::acceptance_matrix::model::MatrixRow;
use crate::acceptance_matrix::producer::ProducerCatalog;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(super) fn validate(
    root: &Path,
    producer_ids: &BTreeSet<String>,
    ci_job_by_producer: &BTreeMap<String, String>,
) -> Result<()> {
    registry::load(root, producer_ids, ci_job_by_producer).map(|_| ())
}

pub(super) fn run(
    root: &Path,
    args: &[String],
    producers: &ProducerCatalog,
    rows: &[MatrixRow],
) -> Result<()> {
    let loaded = registry::load(root, &producers.producer_ids, &producers.ci_job_by_producer)?;
    let matrix = fs::read(root.join(MATRIX_PATH))
        .with_context(|| format!("acceptance-impact: failed to fingerprint {MATRIX_PATH}"))?;
    let args = plan::PlanArgs::parse(args)?;
    let plan = plan::build(
        root,
        &loaded.registry,
        model::InputFingerprints {
            impact_registry: loaded.impact_fingerprint,
            producer_registry: producers.registry_fingerprint.clone(),
            acceptance_matrix: format!("sha256:{:x}", Sha256::digest(&matrix)),
        },
        args,
        &producers.evidence_by_producer,
        rows,
    )?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

pub(super) fn run_backlog(
    rows: &[MatrixRow],
    executable_evidence_ids: &BTreeSet<String>,
) -> Result<()> {
    println!("{}", backlog::render(rows, executable_evidence_ids)?);
    Ok(())
}

pub(super) fn run_shadow(
    root: &Path,
    args: &[String],
    producers: &ProducerCatalog,
    rows: &[MatrixRow],
) -> Result<()> {
    let loaded = registry::load(root, &producers.producer_ids, &producers.ci_job_by_producer)?;
    let matrix = fs::read(root.join(MATRIX_PATH)).with_context(|| {
        format!("acceptance-impact-shadow: failed to fingerprint {MATRIX_PATH}")
    })?;
    let report = shadow::render(
        root,
        &loaded.registry,
        model::InputFingerprints {
            impact_registry: loaded.impact_fingerprint,
            producer_registry: producers.registry_fingerprint.clone(),
            acceptance_matrix: format!("sha256:{:x}", Sha256::digest(&matrix)),
        },
        args,
        &producers.evidence_by_producer,
        rows,
    )?;
    println!("{report}");
    Ok(())
}
