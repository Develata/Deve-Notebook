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

    let identity = yaml_mapping_block(&candidate, 2, "identity")?;
    require_ordered_text(
        &identity,
        &[
            "Checkout exact dispatch commit",
            "ref: ${{ github.sha }}",
            "Lock candidate version and commit",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "scripts/check-release-version-match.sh \"v$VERSION\"",
        ],
        "release-candidate.yml identity job",
    )?;
    let contracts = yaml_mapping_block(&candidate, 2, "contract-static")?;
    require_ordered_text(
        &contracts,
        &[
            "Cache Cargo source inputs",
            "Fetch locked Cargo inputs",
            "release-freeze verify-candidate",
            "deve_baseline -- all",
            "plan-coverage.sh --check-reverse-coverage",
        ],
        "release-candidate.yml static contract job",
    )?;
    let web_dist = yaml_mapping_block(&candidate, 2, "web-dist")?;
    require_ordered_text(
        &web_dist,
        &[
            "Install Web projection build tool",
            "DEVE_NATIVE_INSTALL_TAURI_CLI: \"0\"",
            "scripts/install-native-target-host-tools.sh",
            "Build exact Web projection once",
            "scripts/build-web-dist-ci.sh",
            "web-dist-artifact.mjs seal",
            "deve-candidate-web-dist-${{ github.sha }}",
            "include-hidden-files: true",
        ],
        "release-candidate.yml Web dist job",
    )?;
    let quality = yaml_mapping_block(&candidate, 2, "rust-quality")?;
    require_ordered_text(
        &quality,
        &[
            "Install Linux native compile dependencies",
            "cargo fmt --all -- --check",
            "cargo clippy --locked --all-targets -- -D warnings",
            "cargo check --locked -p deve_web --target wasm32-unknown-unknown",
        ],
        "release-candidate.yml Rust quality job",
    )?;
    let workspace_tests = yaml_mapping_block(&candidate, 2, "workspace-tests")?;
    require_ordered_text(
        &workspace_tests,
        &[
            "Download immutable exact Web projection",
            "web-dist-artifact.mjs verify",
            "cargo test --locked",
        ],
        "release-candidate.yml workspace test job",
    )?;
    let full_baseline = yaml_mapping_block(&candidate, 2, "full-baseline")?;
    require_ordered_text(
        &full_baseline,
        &[
            "Download immutable exact Web projection",
            "web-dist-artifact.mjs verify",
            "deve_baseline -- full",
        ],
        "release-candidate.yml full baseline job",
    )?;

    let docker = yaml_mapping_block(&candidate, 2, "docker-linux-amd64-build")?;
    require_ordered_text(
        &docker,
        &[
            "Verify exact checkout",
            "GITHUB_RUN_ATTEMPT",
            "dispatch a fresh run instead of rerunning",
            "Download immutable exact Web projection",
            "web-dist-artifact.mjs verify",
            "Build linux/amd64 image once",
            "Dockerfile.candidate",
            "--platform linux/amd64",
            "docker image inspect --format '{{.Os}}/{{.Architecture}}'",
            "docker save --output",
            "Generate exact Docker image SPDX 2.3 SBOM",
            "deve-docker-candidate-${{ github.sha }}",
        ],
        "release-candidate.yml Docker job",
    )?;
    if docker.matches("docker buildx build").count() != 1 {
        bail!("release-baseline-check: candidate Docker image must be built exactly once");
    }
    let docker_smoke = yaml_mapping_block(&candidate, 2, "docker-linux-amd64-smoke")?;
    require_ordered_text(
        &docker_smoke,
        &[
            "Download immutable Docker candidate input",
            "docker load --input",
            "docker image inspect --format '{{.Id}}'",
            "--producer docker.multiclient-product",
            "--producer docker.remote-import-browser",
            "--producer docker.p2p-gap-recovery",
        ],
        "release-candidate.yml Docker smoke job",
    )?;

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
    require_text(
        &native_call,
        "web_dist_artifact: deve-candidate-web-dist-${{ github.sha }}",
        "release-candidate.yml native Web dist input",
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
            "require_count windows \"$windows\" 3",
            "require_count macos \"$macos\" 2",
            "require_count android \"$android\" 2",
            "require_count docker \"$docker\" 3",
            "sha256sum -c windows-candidate-input.sha256",
            "sha256sum -c macos-candidate-input.sha256",
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
