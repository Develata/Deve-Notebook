//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix
//!   - 18_release#artifact-identity-and-integrity

use super::read_workflow;
use super::text::{has_v_tag_trigger, require_ordered_text, require_text};
use anyhow::{Result, bail};
use std::path::Path;

pub(super) fn check(root: &Path) -> Result<()> {
    let aggregate = read_workflow(root, "acceptance-aggregate.yml")?;
    require_ordered_text(
        &aggregate,
        &[
            "receipt_run_ids:",
            "exactly one release-candidate run ID is required",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            ".github/workflows/release-candidate.yml",
            "candidate run attempt must be 1; dispatch a fresh release candidate",
            "deve-release-candidate-$GITHUB_SHA",
            "deve-acceptance-receipts-*",
            "Verify sealed artifacts and GitHub attestations",
            "candidate manifest run attempt does not match Actions metadata",
            "provenance-attestation",
            "docker-sbom-attestation",
            "scripts/check-release-candidate-bundle.sh",
            "acceptance-collect",
            "acceptance-matrix",
            "--tag-ready",
            "deve-release-sealed-${{ github.sha }}",
        ],
        "acceptance-aggregate.yml",
    )?;
    if has_v_tag_trigger(&aggregate) || aggregate.contains("  push:") {
        bail!("release-baseline-check: acceptance aggregate must be manual-only");
    }
    require_text(
        &aggregate,
        "persist-credentials: false",
        "acceptance aggregate checkout",
    )?;
    Ok(())
}
