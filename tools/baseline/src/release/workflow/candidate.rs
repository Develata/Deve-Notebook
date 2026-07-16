//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity

use super::text::{
    has_v_tag_trigger, require_exact_mapping_keys, require_ordered_text, require_text,
    yaml_mapping_block,
};
use super::{ANDROID_SIGNING_SECRETS, read_workflow};
use anyhow::{Result, bail};
use std::path::Path;

pub(super) fn check(root: &Path) -> Result<()> {
    let candidate = read_workflow(root, "release-candidate.yml")?;
    require_text(&candidate, "workflow_dispatch:", "release-candidate.yml")?;
    require_text(&candidate, "version:", "release-candidate.yml input")?;
    require_text(
        &candidate,
        "group: release-candidate-${{ github.sha }}-${{ inputs.version }}",
        "release-candidate.yml concurrency",
    )?;
    if has_v_tag_trigger(&candidate) || candidate.contains("  push:") {
        bail!("release-baseline-check: release-candidate.yml must be manual-only");
    }
    let candidate_header = candidate.split("\njobs:").next().unwrap_or(&candidate);
    for forbidden in [
        "artifact-metadata: write",
        "attestations: write",
        "id-token: write",
    ] {
        if candidate_header.contains(forbidden) {
            bail!(
                "release-baseline-check: release-candidate.yml top-level permissions overgrant {forbidden}"
            );
        }
    }

    let validate = yaml_mapping_block(&candidate, 2, "validate")?;
    require_ordered_text(
        &validate,
        &[
            "Checkout exact dispatch commit",
            "ref: ${{ github.sha }}",
            "Lock candidate version and commit",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "scripts/check-release-version-match.sh \"v$VERSION\"",
            "Install Linux native preflight dependencies",
            "libwebkit2gtk-4.1-dev",
            "libgtk-3-dev",
            "librsvg2-dev",
            "libayatana-appindicator3-dev",
            "cargo fmt --all -- --check",
            "Install Web projection build tool",
            "DEVE_NATIVE_INSTALL_TAURI_CLI: \"0\"",
            "scripts/install-native-target-host-tools.sh",
            "Build exact Web projection",
            "scripts/build-web-dist-ci.sh",
            "cargo run --locked --quiet -p deve_baseline -- all",
            "cargo run --locked --quiet -p deve_baseline -- full",
            "cargo clippy --locked --all-targets -- -D warnings",
            "cargo test --locked",
        ],
        "release-candidate.yml validate job",
    )?;

    let docker = yaml_mapping_block(&candidate, 2, "docker-linux-amd64")?;
    require_ordered_text(
        &docker,
        &[
            "Verify exact checkout",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Build linux/amd64 image once",
            "--platform linux/amd64",
            "docker image inspect --format '{{.Os}}/{{.Architecture}}'",
            "--producer docker.multiclient-product",
            "--producer docker.p2p-gap-recovery",
            "--producer security.tag-ready-audit",
            "docker save --output",
            "docker image rm",
            "docker load --input",
            "Generate exact Docker image SPDX 2.3 SBOM",
            "deve-docker-candidate-${{ github.sha }}",
        ],
        "release-candidate.yml Docker job",
    )?;
    if docker.matches("docker buildx build").count() != 1 {
        bail!("release-baseline-check: candidate Docker image must be built exactly once");
    }

    let native_call = yaml_mapping_block(&candidate, 2, "native")?;
    require_text(
        &native_call,
        "uses: ./.github/workflows/release-native.yml",
        "release-candidate.yml native call",
    )?;
    require_text(
        &native_call,
        "candidate_head: ${{ github.sha }}",
        "release-candidate.yml native call",
    )?;
    if native_call.contains("secrets: inherit") {
        bail!("release-baseline-check: candidate must not inherit all native secrets");
    }
    let secret_map = yaml_mapping_block(&native_call, 4, "secrets")?;
    require_exact_mapping_keys(
        &secret_map,
        6,
        &ANDROID_SIGNING_SECRETS,
        "release-candidate.yml native secret map",
    )?;

    let assemble = yaml_mapping_block(&candidate, 2, "assemble")?;
    require_ordered_text(
        &assemble,
        &[
            "Checkout exact candidate",
            "Lock immutable candidate attempt",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Generate exact source SPDX 2.3 SBOM before artifact download",
            "Download exact Windows candidate",
            "Download exact macOS candidate",
            "Download exact Android candidate",
            "Download exact Docker candidate",
            "Materialize a strict candidate tree",
            "require_count windows",
            "require_count macos",
            "require_count android",
            "require_count docker",
            "actions/attest@a1948c3f048ba23858d222213b7c278aabede763",
            "sbom-path:",
            "create-storage-record: false",
            "Seal the returned attestation bundles",
            "--provenance-bundle",
            "--docker-sbom-bundle",
            "release-candidate assemble",
            "release-candidate verify",
            "--producer release.candidate-bundle",
            "deve-release-candidate-${{ github.sha }}",
        ],
        "release-candidate.yml assemble job",
    )?;
    for permission in [
        "artifact-metadata: write",
        "attestations: write",
        "id-token: write",
    ] {
        require_text(
            &assemble,
            permission,
            "release-candidate.yml assemble permissions",
        )?;
    }
    Ok(())
}
