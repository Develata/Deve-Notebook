//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!
use super::manifest::ArtifactRole;
use super::paths::validate_relative_path;
use anyhow::{Result, bail};
use std::collections::BTreeSet;
use std::path::PathBuf;

const OUTPUT_NAME: &str = "release-candidate.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Action {
    Assemble,
    Verify,
}

#[derive(Clone, Debug)]
pub(super) struct ArtifactInput {
    pub role: ArtifactRole,
    pub path: String,
}

#[derive(Clone, Debug)]
pub(super) struct CandidateArgs {
    pub candidate_dir: PathBuf,
    pub output: String,
    pub head_sha: String,
    pub version: String,
    pub workflow_path: String,
    pub run_id: u64,
    pub run_attempt: u64,
    pub docker_image_id: String,
    pub android_signer_sha256: String,
    pub artifacts: Vec<ArtifactInput>,
}

impl CandidateArgs {
    pub(super) fn parse(args: &[String]) -> Result<(Action, Self)> {
        let (action, rest) = match args.split_first() {
            Some((action, rest)) if action == "assemble" => (Action::Assemble, rest),
            Some((action, rest)) if action == "verify" => (Action::Verify, rest),
            _ => bail!(usage()),
        };

        let mut parser = Parser::new(rest);
        let candidate_dir = PathBuf::from(parser.required("--candidate-dir")?);
        let output = parser.required("--output")?;
        if output != OUTPUT_NAME {
            bail!("--output must be exactly {OUTPUT_NAME}");
        }
        validate_relative_path(&output)?;

        let head_sha = normalize_head(&parser.required("--head")?)?;
        let version = validate_version(parser.required("--version")?)?;
        let workflow_path = parser.required("--workflow-path")?;
        validate_relative_path(&workflow_path)?;
        if !workflow_path.starts_with(".github/workflows/")
            || !(workflow_path.ends_with(".yml") || workflow_path.ends_with(".yaml"))
        {
            bail!("--workflow-path must name a workflow below .github/workflows/");
        }

        let run_id = parse_positive_u64("--run-id", &parser.required("--run-id")?)?;
        let run_attempt = parse_positive_u64("--run-attempt", &parser.required("--run-attempt")?)?;
        let docker_image_id =
            normalize_sha256_identity("--docker-image-id", &parser.required("--docker-image-id")?)?;
        let android_signer_sha256 =
            normalize_certificate_fingerprint(&parser.required("--android-signer-sha256")?)?;

        let mut artifacts = vec![
            parser.artifact("--windows-msi", ArtifactRole::WindowsMsi)?,
            parser.artifact("--windows-nsis", ArtifactRole::WindowsNsis)?,
            parser.artifact("--macos-dmg", ArtifactRole::MacosDmg)?,
            parser.artifact("--android-apk", ArtifactRole::AndroidArm64Apk)?,
            parser.artifact("--docker-archive", ArtifactRole::DockerLinuxAmd64Archive)?,
            parser.artifact("--source-sbom", ArtifactRole::SourceSpdx)?,
            parser.artifact("--image-sbom", ArtifactRole::ImageSpdx)?,
        ];
        artifacts
            .push(parser.artifact("--provenance-bundle", ArtifactRole::ProvenanceAttestation)?);
        artifacts
            .push(parser.artifact("--docker-sbom-bundle", ArtifactRole::DockerSbomAttestation)?);
        parser.finish()?;

        let unique: BTreeSet<_> = artifacts.iter().map(|item| item.path.as_str()).collect();
        if unique.len() != artifacts.len() {
            bail!("artifact paths and attestation paths must be unique");
        }
        let unique_basenames: BTreeSet<_> = artifacts
            .iter()
            .map(|item| {
                item.path
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
            })
            .collect();
        if unique_basenames.len() != artifacts.len() {
            bail!("candidate artifacts must have unique case-insensitive basenames");
        }
        for reserved in [OUTPUT_NAME, "SHA256SUMS", "release-candidate-SHA256SUMS"] {
            if unique_basenames.contains(&reserved.to_ascii_lowercase()) {
                bail!(
                    "artifact basename collides with generated candidate control file {reserved}"
                );
            }
        }

        Ok((
            action,
            Self {
                candidate_dir,
                output,
                head_sha,
                version,
                workflow_path,
                run_id,
                run_attempt,
                docker_image_id,
                android_signer_sha256,
                artifacts,
            },
        ))
    }
}

struct Parser<'a> {
    args: &'a [String],
    consumed: BTreeSet<usize>,
}

impl<'a> Parser<'a> {
    fn new(args: &'a [String]) -> Self {
        Self {
            args,
            consumed: BTreeSet::new(),
        }
    }

    fn required(&mut self, flag: &str) -> Result<String> {
        let values = self.take_values(flag)?;
        match values.as_slice() {
            [value] => Ok(value.clone()),
            [] => bail!("missing required argument {flag}"),
            _ => bail!("argument {flag} must occur exactly once"),
        }
    }

    fn artifact(&mut self, flag: &str, role: ArtifactRole) -> Result<ArtifactInput> {
        let path = self.required(flag)?;
        validate_artifact_path(role, &path)?;
        Ok(ArtifactInput { role, path })
    }

    fn take_values(&mut self, flag: &str) -> Result<Vec<String>> {
        let mut values = Vec::new();
        for index in 0..self.args.len() {
            if self.args[index] != flag {
                continue;
            }
            if self.consumed.contains(&index) {
                continue;
            }
            let value_index = index + 1;
            let Some(value) = self.args.get(value_index) else {
                bail!("argument {flag} requires a value");
            };
            if value.starts_with("--") {
                bail!("argument {flag} requires a value");
            }
            self.consumed.insert(index);
            self.consumed.insert(value_index);
            values.push(value.clone());
        }
        Ok(values)
    }

    fn finish(&self) -> Result<()> {
        let unknown: Vec<_> = self
            .args
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.consumed.contains(index))
            .map(|(_, value)| value.as_str())
            .collect();
        if !unknown.is_empty() {
            bail!("unknown or positional release-candidate arguments: {unknown:?}");
        }
        Ok(())
    }
}

fn validate_artifact_path(role: ArtifactRole, path: &str) -> Result<()> {
    validate_relative_path(path)?;
    let name = path
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    for reserved in [OUTPUT_NAME, "SHA256SUMS", "release-candidate-SHA256SUMS"] {
        if name == reserved.to_ascii_lowercase() {
            bail!("artifact basename collides with generated candidate control file {reserved}");
        }
    }
    match role {
        ArtifactRole::WindowsMsi if !name.ends_with(".msi") => {
            bail!("--windows-msi must identify one .msi file")
        }
        ArtifactRole::WindowsNsis if !name.ends_with(".exe") => {
            bail!("--windows-nsis must identify one NSIS .exe file")
        }
        ArtifactRole::MacosDmg if !name.ends_with(".dmg") => {
            bail!("--macos-dmg must identify one .dmg file")
        }
        ArtifactRole::AndroidArm64Apk
            if !name.ends_with("-android-arm64.apk") || name.contains("unsigned") =>
        {
            bail!("--android-apk must identify a signed *-android-arm64.apk file")
        }
        ArtifactRole::DockerLinuxAmd64Archive
            if !(name.ends_with(".tar") || name.ends_with(".tar.zst")) =>
        {
            bail!("--docker-archive must identify one .tar or .tar.zst archive")
        }
        ArtifactRole::SourceSpdx if !name.ends_with(".spdx.json") || !name.contains("source") => {
            bail!("--source-sbom must identify a source*.spdx.json document")
        }
        ArtifactRole::ImageSpdx
            if !name.ends_with(".spdx.json")
                || !(name.contains("image") || name.contains("docker")) =>
        {
            bail!("--image-sbom must identify an image/docker *.spdx.json document")
        }
        ArtifactRole::ProvenanceAttestation | ArtifactRole::DockerSbomAttestation
            if !(name.ends_with(".json")
                || name.ends_with(".jsonl")
                || name.ends_with(".bundle")) =>
        {
            bail!("attestation arguments must identify a JSON or bundle declaration")
        }
        _ => {}
    }
    Ok(())
}

fn normalize_head(value: &str) -> Result<String> {
    let normalized = value.to_ascii_lowercase();
    if normalized.len() != 40 || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("--head must be a 40-character Git object ID");
    }
    Ok(normalized)
}

fn validate_version(value: String) -> Result<String> {
    if value.is_empty()
        || value.starts_with('v')
        || value.chars().any(|character| {
            character.is_whitespace()
                || character.is_control()
                || character == '/'
                || character == '\\'
        })
    {
        bail!("--version must be a non-empty literal workspace version without a v prefix");
    }
    Ok(value)
}

fn parse_positive_u64(flag: &str, value: &str) -> Result<u64> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("{flag} must be a positive integer"))?;
    if parsed == 0 {
        bail!("{flag} must be a positive integer");
    }
    Ok(parsed)
}

fn normalize_sha256_identity(flag: &str, value: &str) -> Result<String> {
    let Some(digest) = value.strip_prefix("sha256:") else {
        bail!("{flag} must use sha256:<64 lowercase hex> form");
    };
    if digest.len() != 64
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        || digest
            .chars()
            .any(|character| character.is_ascii_uppercase())
    {
        bail!("{flag} must use sha256:<64 lowercase hex> form");
    }
    Ok(value.to_owned())
}

fn normalize_certificate_fingerprint(value: &str) -> Result<String> {
    let value = value.strip_prefix("sha256:").unwrap_or(value);
    let digest: String = value
        .chars()
        .filter(|character| *character != ':')
        .collect::<String>()
        .to_ascii_lowercase();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("--android-signer-sha256 must be a SHA-256 certificate fingerprint");
    }
    Ok(format!("sha256:{digest}"))
}

fn usage() -> &'static str {
    "Usage: deve_baseline release-candidate <assemble|verify> \\
  --candidate-dir <dir> --output release-candidate.json \\
  --head <git-sha> --version <workspace-version> \\
  --workflow-path .github/workflows/release-candidate.yml \\
  --run-id <id> --run-attempt <attempt> \\
  --docker-image-id sha256:<digest> \\
  --android-signer-sha256 <certificate-fingerprint> \\
  --windows-msi <relative-path> --windows-nsis <relative-path> \\
  --macos-dmg <relative-path> --android-apk <relative-path> \\
  --docker-archive <relative-path> --source-sbom <relative-path> \\
  --image-sbom <relative-path> \\
  --provenance-bundle <relative-path> --docker-sbom-bundle <relative-path>"
}
