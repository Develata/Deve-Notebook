//! Validated change-set inputs for deterministic impact planning.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use anyhow::{Context, Result, bail};
use std::path::{Component, Path};
use std::process::Command;

pub(in crate::acceptance_matrix::impact) struct PlanArgs {
    pub(super) profile: String,
    pub(super) base: Option<String>,
    pub(super) head: Option<String>,
    pub(super) changed_files: Vec<String>,
}

impl PlanArgs {
    pub(in crate::acceptance_matrix::impact) fn parse(args: &[String]) -> Result<Self> {
        let mut profile = None;
        let mut base = None;
        let mut head = None;
        let mut changed_files = Vec::new();
        let mut index = 0usize;
        while index < args.len() {
            let option = args[index].as_str();
            index += 1;
            let value = args
                .get(index)
                .with_context(|| format!("acceptance-impact: {option} requires a value"))?;
            index += 1;
            match option {
                "--profile" if profile.is_none() => profile = Some(value.clone()),
                "--base" if base.is_none() => base = Some(validate_revision(value)?),
                "--head" if head.is_none() => head = Some(validate_revision(value)?),
                "--changed-file" => changed_files.push(validate_changed_path(value)?),
                _ => bail!("acceptance-impact: unknown or repeated option {option}"),
            }
        }
        if base.is_some() != head.is_some() {
            bail!("acceptance-impact: --base and --head must be supplied together");
        }
        if base.is_some() && !changed_files.is_empty() {
            bail!("acceptance-impact: revisions and --changed-file may not be mixed");
        }
        Ok(Self {
            profile: profile.context("acceptance-impact: --profile is required")?,
            base,
            head,
            changed_files,
        })
    }
}

pub(super) fn git_changed_files(root: &Path, base: &str, head: &str) -> Result<Vec<String>> {
    let range = format!("{base}...{head}");
    let output = Command::new("git")
        .args(["diff", "--name-only", "-z", "--no-renames", &range, "--"])
        .current_dir(root)
        .output()
        .context("acceptance-impact: failed to compute git change set")?;
    if !output.status.success() {
        bail!("acceptance-impact: git diff failed for {range}");
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            let path = std::str::from_utf8(path)
                .context("acceptance-impact: changed path is not UTF-8")?;
            validate_changed_path(path)
        })
        .collect()
}

pub(super) fn validate_changed_path(value: &str) -> Result<String> {
    if value.is_empty()
        || value.contains('\\')
        || Path::new(value)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("acceptance-impact: invalid changed path {value:?}");
    }
    Ok(value.to_owned())
}

pub(in crate::acceptance_matrix::impact) fn validate_revision(value: &str) -> Result<String> {
    if value.is_empty()
        || value.starts_with('-')
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '/' | '~' | '^'))
    {
        bail!("acceptance-impact: invalid git revision {value:?}");
    }
    Ok(value.to_owned())
}
