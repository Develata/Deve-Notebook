//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!   - 18_release#release-versioning
//!
//! Developer-only verification of the first-tag release freeze. The registry
//! owns the selected version and paths; this module proves that product version
//! surfaces, candidate assembly, and tag promotion still project that authority.

mod candidate;
mod model;
mod workflows;

use crate::context::BaselineContext;
use anyhow::{Context, Result, bail, ensure};
use candidate::{controls, fixed_artifacts, validate_candidate_contract};
use model::{ArtifactPath, ReleaseFreeze};
use regex::Regex;
use semver::Version;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use workflows::validate_workflows;

const REGISTRY_PATH: &str = "docs/registry/release-freeze.json";
const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) use candidate::validate_candidate_paths;

pub fn run(args: &[String]) -> Result<()> {
    if args != ["verify"] {
        bail!("Usage: deve_baseline release-freeze verify");
    }
    verify()
}

pub(crate) fn verify() -> Result<()> {
    let ctx = BaselineContext::new("release-freeze-check")?;
    verify_root(ctx.root())?;
    ctx.ok();
    Ok(())
}

fn verify_root(root: &Path) -> Result<()> {
    let registry_bytes = read_bounded(root.join(REGISTRY_PATH), MAX_TEXT_BYTES)?;
    let registry: ReleaseFreeze =
        serde_json::from_slice(&registry_bytes).context("parse typed release freeze registry")?;
    validate_registry(&registry)?;
    validate_version_surfaces(root, &registry)?;
    validate_candidate_contract(&registry)?;
    validate_workflows(root, &registry)
}

fn validate_registry(registry: &ReleaseFreeze) -> Result<()> {
    ensure!(registry.schema == 1, "release freeze schema must be 1");
    let version = Version::parse(&registry.release.version)
        .context("release freeze version must be valid SemVer")?;
    ensure!(
        version == Version::new(0, 1, 0),
        "first public tag version must remain 0.1.0"
    );
    ensure!(
        registry.release.tag == format!("v{version}"),
        "release freeze tag must be v<version>"
    );
    ensure!(
        registry.release.channel == "public-preview",
        "first release channel must be public-preview"
    );

    let artifacts = &registry.artifacts;
    require_public(
        "Windows x64 MSI",
        &artifacts.windows_x64_msi,
        "-windows-x64.msi",
    )?;
    require_public(
        "Windows x64 NSIS",
        &artifacts.windows_x64_nsis,
        "-windows-x64-setup.exe",
    )?;
    ensure!(
        artifacts.macos_host_dmg.public && !artifacts.macos_host_dmg.universal,
        "macOS host DMG must be public and non-universal"
    );
    let expected_macos = BTreeSet::from([
        "artifacts/macos/deve-notebook-{version}-macos-arm64.dmg",
        "artifacts/macos/deve-notebook-{version}-macos-x64.dmg",
    ]);
    let actual_macos: BTreeSet<_> = artifacts
        .macos_host_dmg
        .one_of
        .iter()
        .map(String::as_str)
        .collect();
    ensure!(
        actual_macos == expected_macos && artifacts.macos_host_dmg.one_of.len() == 2,
        "macOS host DMG one_of must contain exactly x64 and arm64"
    );
    ensure!(
        artifacts.android_arm64_apk.public && artifacts.android_arm64_apk.signed,
        "Android ARM64 APK must be public and signed"
    );
    require_suffix(
        "Android ARM64 APK",
        &artifacts.android_arm64_apk.path,
        "-android-arm64.apk",
    )?;
    ensure!(
        !artifacts.docker_linux_amd64_archive.public,
        "Docker archive must remain candidate-internal"
    );
    require_suffix(
        "Docker linux/amd64 archive",
        &artifacts.docker_linux_amd64_archive.path,
        "-linux-amd64.tar",
    )?;
    for (label, artifact) in [
        ("source SPDX", &artifacts.source_spdx),
        ("image SPDX", &artifacts.image_spdx),
    ] {
        ensure!(artifact.public, "{label} must be public");
        ensure!(
            artifact.spdx_version == "SPDX-2.3",
            "{label} must use SPDX-2.3"
        );
        require_suffix(label, &artifact.path, ".spdx.json")?;
    }
    ensure!(
        artifacts.provenance_bundle.public && artifacts.docker_sbom_bundle.public,
        "provenance and Docker SBOM bundles must be public"
    );

    ensure!(
        registry.controls.release_candidate.public
            && !registry.controls.candidate_checksums.public
            && registry.controls.public_checksums.public,
        "candidate control public policy is invalid"
    );
    ensure!(
        registry.controls.release_candidate.path == "release-candidate.json"
            && registry.controls.candidate_checksums.path == "release-candidate-SHA256SUMS"
            && registry.controls.public_checksums.path == "SHA256SUMS",
        "candidate control file identities are invalid"
    );

    validate_paths_and_basenames(registry)?;
    let expected_exclusions = BTreeSet::from([
        "ios",
        "linux-native-desktop",
        "macos-universal",
        "physical-device-readiness",
        "stable-data-compatibility",
        "standalone-cli",
        "store-readiness",
        "windows-macos-signed-notarized-claim",
    ]);
    let actual_exclusions: BTreeSet<_> = registry.excluded.iter().map(String::as_str).collect();
    ensure!(
        actual_exclusions == expected_exclusions
            && actual_exclusions.len() == registry.excluded.len(),
        "release freeze exclusions are incomplete, duplicated, or unsupported"
    );
    Ok(())
}

fn validate_paths_and_basenames(registry: &ReleaseFreeze) -> Result<()> {
    let version = registry.release.version.as_str();
    let fixed = fixed_artifacts(registry);
    for artifact in &fixed {
        validate_template(artifact.label, artifact.path, version)?;
        let expected_placeholders = match artifact.role {
            "windows-msi" | "windows-nsis" | "android-arm64-apk" | "docker-linux-amd64-archive" => {
                1
            }
            _ => 0,
        };
        ensure!(
            artifact.path.matches("{version}").count() == expected_placeholders,
            "{} path has an invalid version placeholder count",
            artifact.label
        );
    }
    for path in &registry.artifacts.macos_host_dmg.one_of {
        validate_template("macOS host DMG", path, version)?;
    }
    for (label, control) in controls(registry) {
        validate_template(label, &control.path, version)?;
    }

    for macos in &registry.artifacts.macos_host_dmg.one_of {
        let mut basenames = BTreeSet::new();
        for path in fixed
            .iter()
            .map(|artifact| artifact.path)
            .chain(std::iter::once(macos.as_str()))
            .chain(controls(registry).map(|(_, control)| control.path.as_str()))
        {
            let rendered = path.replace("{version}", version);
            let basename = rendered
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            ensure!(
                !basename.is_empty() && basenames.insert(basename),
                "release artifacts and controls must have unique case-insensitive basenames"
            );
        }
    }
    Ok(())
}

fn validate_version_surfaces(root: &Path, registry: &ReleaseFreeze) -> Result<()> {
    let expected = registry.release.version.as_str();
    let cargo = read_text(root.join("Cargo.toml"))?;
    ensure!(
        toml_string(&cargo, "workspace.package", "version").as_deref() == Some(expected),
        "workspace package version does not match release freeze"
    );
    for path in [
        "apps/desktop/tauri.conf.json",
        "apps/mobile/tauri.conf.json",
    ] {
        let value: Value = serde_json::from_str(&read_text(root.join(path))?)
            .with_context(|| format!("parse {path}"))?;
        ensure!(
            value.get("version").and_then(Value::as_str) == Some(expected),
            "{path} version does not match release freeze"
        );
    }
    let android_path = "apps/mobile/gen/android/app/build.gradle.kts";
    let android = read_text(root.join(android_path))?;
    let fallback = android_version_fallback(&android)?;
    ensure!(
        fallback == expected,
        "{android_path} versionName fallback does not match release freeze"
    );
    Ok(())
}

fn android_version_fallback(content: &str) -> Result<String> {
    let pattern = Regex::new(
        r#"(?m)^\s*versionName\s*=\s*tauriProperties\.getProperty\(\s*"tauri\.android\.versionName"\s*,\s*"([^"]+)"\s*\)\s*$"#,
    )?;
    let values = pattern
        .captures_iter(content)
        .map(|captures| captures[1].to_owned())
        .collect::<Vec<_>>();
    ensure!(
        values.len() == 1,
        "Android versionName fallback must have exactly one executable assignment"
    );
    Ok(values.into_iter().next().expect("one fallback"))
}

fn require_public(label: &str, artifact: &ArtifactPath, suffix: &str) -> Result<()> {
    ensure!(artifact.public, "{label} must be public");
    require_suffix(label, &artifact.path, suffix)
}

fn require_suffix(label: &str, path: &str, suffix: &str) -> Result<()> {
    ensure!(
        path.ends_with(suffix),
        "{label} path must end with {suffix}"
    );
    Ok(())
}

fn validate_template(label: &str, template: &str, version: &str) -> Result<()> {
    ensure!(
        !template.is_empty()
            && !template.starts_with('/')
            && !template.ends_with('/')
            && !template.contains('\\')
            && !template.contains(':')
            && !template.contains("//"),
        "{label} path is not a normalized relative path"
    );
    ensure!(
        !template.chars().any(char::is_control),
        "{label} path contains a control character"
    );
    let without_version = template.replace("{version}", "");
    ensure!(
        !without_version.contains('{') && !without_version.contains('}'),
        "{label} path contains an unsupported placeholder"
    );
    let rendered = template.replace("{version}", version);
    ensure!(
        rendered
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != ".."),
        "{label} path contains an unsafe component"
    );
    Ok(())
}

fn toml_string(content: &str, section: &str, key: &str) -> Option<String> {
    let mut current = "";
    for raw in content.lines() {
        let line = raw.split('#').next()?.trim();
        if line.starts_with('[') && line.ends_with(']') {
            current = line.trim_matches(['[', ']']);
            continue;
        }
        if current != section {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() == key {
            return value
                .trim()
                .strip_prefix('"')?
                .strip_suffix('"')
                .map(str::to_owned);
        }
    }
    None
}

fn read_text(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    String::from_utf8(read_bounded(path, MAX_TEXT_BYTES)?)
        .with_context(|| format!("{} must be UTF-8", path.display()))
}

fn read_bounded(path: impl AsRef<Path>, maximum: u64) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("inspect release freeze input {}", path.display()))?;
    ensure!(
        metadata.file_type().is_file() && metadata.len() <= maximum,
        "release freeze input {} must be a regular file within {maximum} bytes",
        path.display()
    );
    let file = File::open(path)
        .with_context(|| format!("open release freeze input {}", path.display()))?;
    let opened = file
        .metadata()
        .with_context(|| format!("inspect opened release freeze input {}", path.display()))?;
    ensure!(
        opened.is_file() && opened.len() <= maximum,
        "opened release freeze input {} must be a regular file within {maximum} bytes",
        path.display()
    );
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read release freeze input {}", path.display()))?;
    ensure!(
        bytes.len() as u64 <= maximum,
        "release freeze input {} grew beyond {maximum} bytes",
        path.display()
    );
    Ok(bytes)
}

#[cfg(test)]
mod tests;
