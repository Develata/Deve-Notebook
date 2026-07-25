//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!
//! Candidate assembler projections of the typed release freeze.

use super::model::{ArtifactPath, ReleaseFreeze};
use super::{MAX_TEXT_BYTES, REGISTRY_PATH, read_bounded, validate_registry};
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate_candidate_paths<'a>(
    version: &str,
    artifacts: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<()> {
    let root = crate::workspace_root::repo_root()?;
    let registry_bytes = read_bounded(root.join(REGISTRY_PATH), MAX_TEXT_BYTES)?;
    let registry: ReleaseFreeze =
        serde_json::from_slice(&registry_bytes).context("parse typed release freeze registry")?;
    validate_registry(&registry)?;
    ensure!(
        version == registry.release.version,
        "release candidate version does not match release freeze"
    );

    let mut actual = BTreeMap::new();
    for (role, path) in artifacts {
        ensure!(
            actual.insert(role, path).is_none(),
            "release candidate role {role} occurs more than once"
        );
    }
    let expected_roles: BTreeSet<_> = fixed_artifacts(&registry)
        .iter()
        .map(|artifact| artifact.role)
        .chain(std::iter::once("macos-dmg"))
        .collect();
    ensure!(
        actual.keys().copied().collect::<BTreeSet<_>>() == expected_roles,
        "release candidate roles do not match release freeze"
    );
    for artifact in fixed_artifacts(&registry) {
        let expected = artifact.path.replace("{version}", version);
        ensure!(
            actual.get(artifact.role).copied() == Some(expected.as_str()),
            "release candidate {} path does not match release freeze",
            artifact.label
        );
    }
    let allowed_macos: BTreeSet<_> = registry
        .artifacts
        .macos_host_dmg
        .one_of
        .iter()
        .map(|path| path.replace("{version}", version))
        .collect();
    ensure!(
        actual
            .get("macos-dmg")
            .is_some_and(|path| allowed_macos.contains(*path)),
        "release candidate macOS DMG path is outside the frozen host-architecture one-of"
    );
    Ok(())
}

pub(super) fn validate_candidate_contract(registry: &ReleaseFreeze) -> Result<()> {
    let expected_roles: BTreeMap<_, _> = fixed_artifacts(registry)
        .into_iter()
        .map(|artifact| (artifact.role, artifact.public))
        .chain(std::iter::once((
            "macos-dmg",
            registry.artifacts.macos_host_dmg.public,
        )))
        .collect();
    ensure!(
        expected_roles == crate::release_candidate::artifact_role_contract(),
        "release candidate artifact role/public policy does not match release freeze"
    );
    let expected_controls: BTreeMap<_, _> = controls(registry)
        .map(|(_, control)| (control.path.as_str(), control.public))
        .collect();
    ensure!(
        expected_controls == crate::release_candidate::control_contract(),
        "release candidate control/public policy does not match release freeze"
    );
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct FrozenArtifactRef<'a> {
    pub label: &'static str,
    pub path: &'a str,
    pub public: bool,
    pub role: &'static str,
}

pub(super) fn fixed_artifacts(registry: &ReleaseFreeze) -> Vec<FrozenArtifactRef<'_>> {
    let artifacts = &registry.artifacts;
    vec![
        FrozenArtifactRef {
            label: "Windows x64 MSI",
            path: &artifacts.windows_x64_msi.path,
            public: artifacts.windows_x64_msi.public,
            role: "windows-msi",
        },
        FrozenArtifactRef {
            label: "Windows x64 NSIS",
            path: &artifacts.windows_x64_nsis.path,
            public: artifacts.windows_x64_nsis.public,
            role: "windows-nsis",
        },
        FrozenArtifactRef {
            label: "Android ARM64 APK",
            path: &artifacts.android_arm64_apk.path,
            public: artifacts.android_arm64_apk.public,
            role: "android-arm64-apk",
        },
        FrozenArtifactRef {
            label: "Docker linux/amd64 archive",
            path: &artifacts.docker_linux_amd64_archive.path,
            public: artifacts.docker_linux_amd64_archive.public,
            role: "docker-linux-amd64-archive",
        },
        FrozenArtifactRef {
            label: "source SPDX",
            path: &artifacts.source_spdx.path,
            public: artifacts.source_spdx.public,
            role: "source-spdx",
        },
        FrozenArtifactRef {
            label: "image SPDX",
            path: &artifacts.image_spdx.path,
            public: artifacts.image_spdx.public,
            role: "image-spdx",
        },
        FrozenArtifactRef {
            label: "provenance bundle",
            path: &artifacts.provenance_bundle.path,
            public: artifacts.provenance_bundle.public,
            role: "provenance-attestation",
        },
        FrozenArtifactRef {
            label: "Docker SBOM bundle",
            path: &artifacts.docker_sbom_bundle.path,
            public: artifacts.docker_sbom_bundle.public,
            role: "docker-sbom-attestation",
        },
    ]
}

pub(super) fn controls(
    registry: &ReleaseFreeze,
) -> impl Iterator<Item = (&'static str, &'_ ArtifactPath)> {
    [
        (
            "release candidate manifest",
            &registry.controls.release_candidate,
        ),
        (
            "candidate-internal checksums",
            &registry.controls.candidate_checksums,
        ),
        ("public checksums", &registry.controls.public_checksums),
    ]
    .into_iter()
}
