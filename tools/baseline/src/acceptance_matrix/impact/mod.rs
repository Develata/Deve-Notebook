//! Shadow-only dependency-aware CI impact planning.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

mod model;
mod plan;
mod registry;

use crate::acceptance_matrix::model::MATRIX_PATH;
use crate::acceptance_matrix::model::MatrixRow;
use crate::acceptance_matrix::producer::ProducerCatalog;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(super) fn validate(root: &Path, producer_ids: &BTreeSet<String>) -> Result<()> {
    registry::load(root, producer_ids).map(|_| ())
}

pub(super) fn run(
    root: &Path,
    args: &[String],
    producers: &ProducerCatalog,
    rows: &[MatrixRow],
) -> Result<()> {
    let loaded = registry::load(root, &producers.producer_ids)?;
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
