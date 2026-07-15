//! plan_ref: docs/plan/18_release.md#release-pipeline-skeleton

use anyhow::{Result, bail};
use semver::{BuildMetadata, Version};

pub fn run(args: &[String]) -> Result<()> {
    let [prior, candidate] = args else {
        bail!("usage: deve_baseline release-version-order <prior-tag> <candidate-tag>");
    };
    let prior = precedence(prior)?;
    let candidate = precedence(candidate)?;
    if candidate <= prior {
        bail!("candidate release precedence {candidate} must be greater than prior latest {prior}");
    }
    println!("release-version-order: {prior} -> {candidate}: ok");
    Ok(())
}

fn precedence(tag: &str) -> Result<Version> {
    let literal = tag.strip_prefix('v').unwrap_or(tag);
    let mut version = Version::parse(literal)
        .map_err(|error| anyhow::anyhow!("invalid SemVer release tag {tag:?}: {error}"))?;
    version.build = BuildMetadata::EMPTY;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::run;

    fn arguments(prior: &str, candidate: &str) -> Vec<String> {
        vec![prior.to_owned(), candidate.to_owned()]
    }

    #[test]
    fn accepts_strict_semver_progression() {
        run(&arguments("v0.9.0", "v1.0.0")).expect("strict progression");
    }

    #[test]
    fn rejects_descendant_with_lower_version() {
        let error = run(&arguments("v2.0.0", "v1.5.0")).expect_err("must reject rollback");
        assert!(format!("{error:#}").contains("must be greater"));
    }

    #[test]
    fn build_metadata_does_not_change_precedence() {
        run(&arguments("v1.0.0+old", "v1.0.1+new")).expect("patch advances");
        run(&arguments("v1.0.0+old", "v1.0.0+new"))
            .expect_err("metadata alone must not advance latest");
    }
}
