//! Exact candidate receipt-artifact upload set validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::{mapping, optional, required, sequence, string};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use yaml_rust2::YamlLoader;

const UPLOAD_ACTION: &str = "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a";
const EXPECTED: [(&str, &str); 11] = [
    (
        "deve-acceptance-receipts-docker-${{ github.sha }}",
        "${{ runner.temp }}/deve-acceptance-docker",
    ),
    (
        "deve-acceptance-receipts-contracts-${{ github.sha }}",
        "${{ runner.temp }}/deve-acceptance-contracts",
    ),
    (
        "deve-acceptance-receipts-release-${{ github.sha }}",
        "${{ runner.temp }}/deve-acceptance-release-candidate",
    ),
    (
        "deve-acceptance-receipts-repo-linux-${{ github.sha }}",
        "${{ runner.temp }}/deve-acceptance-repo-lifecycle-linux",
    ),
    (
        "deve-acceptance-receipts-security-${{ github.sha }}",
        "${{ runner.temp }}/deve-acceptance-security",
    ),
    (
        "deve-acceptance-receipts-github-${{ github.sha }}",
        "${{ runner.temp }}/deve-acceptance-github-pvr",
    ),
    (
        "deve-acceptance-receipts-desktop-local-${{ inputs.candidate_head }}",
        "${{ runner.temp }}/deve-acceptance-desktop-local",
    ),
    (
        "deve-acceptance-receipts-desktop-remote-${{ inputs.candidate_head }}",
        "${{ runner.temp }}/deve-acceptance-desktop-remote",
    ),
    (
        "deve-acceptance-receipts-desktop-macos-${{ inputs.candidate_head }}",
        "${{ runner.temp }}/deve-acceptance-desktop-macos",
    ),
    (
        "deve-acceptance-receipts-android-local-${{ inputs.candidate_head }}",
        "${{ runner.temp }}/deve-acceptance-android-local",
    ),
    (
        "deve-acceptance-receipts-android-remote-${{ inputs.candidate_head }}",
        "${{ runner.temp }}/deve-acceptance-android-remote",
    ),
];

pub(super) fn validate(candidate: &str, native: &str) -> Result<()> {
    let mut actual = BTreeMap::<String, usize>::new();
    for (label, content) in [
        ("release-candidate.yml", candidate),
        ("release-native.yml", native),
    ] {
        collect(label, content, &mut actual)?;
    }
    let unexpected = actual
        .keys()
        .filter(|name| !EXPECTED.iter().any(|(expected, _)| expected == name))
        .cloned()
        .collect::<Vec<_>>();
    let missing = EXPECTED
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !actual.contains_key(*name))
        .collect::<Vec<_>>();
    let duplicate = actual
        .iter()
        .filter(|(_, count)| **count != 1)
        .map(|(name, count)| format!("{name}:{count}"))
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() || !duplicate.is_empty() {
        bail!(
            "acceptance producers: receipt artifact upload set mismatch; missing=[{}] unexpected=[{}] duplicate=[{}]",
            missing.join(","),
            unexpected.join(","),
            duplicate.join(",")
        );
    }
    Ok(())
}

fn collect(label: &str, content: &str, actual: &mut BTreeMap<String, usize>) -> Result<()> {
    let documents = YamlLoader::load_from_str(content)
        .with_context(|| format!("acceptance producers: {label} is not valid YAML"))?;
    let [document] = documents.as_slice() else {
        bail!("acceptance producers: {label} must contain one YAML document");
    };
    let root = mapping(document, label)?;
    let jobs = mapping(required(root, "jobs", label)?, &format!("{label}.jobs"))?;
    for (job_key, job_value) in jobs {
        let job_id = string(job_key, &format!("{label}.jobs key"))?;
        let job = mapping(job_value, &format!("{label}.jobs.{job_id}"))?;
        let Some(steps) = optional(job, "steps") else {
            continue;
        };
        for (index, step_value) in sequence(steps, &format!("{label}.jobs.{job_id}.steps"))?
            .iter()
            .enumerate()
        {
            let path = format!("{label}.jobs.{job_id}.steps[{index}]");
            let step = mapping(step_value, &path)?;
            let Some(with) = optional(step, "with") else {
                continue;
            };
            let with = mapping(with, &format!("{path}.with"))?;
            let Some(name) = optional(with, "name") else {
                continue;
            };
            let name = string(name, &format!("{path}.with.name"))?;
            if !name.starts_with("deve-acceptance-receipts-") {
                continue;
            }
            let uses = string(required(step, "uses", &path)?, &format!("{path}.uses"))?;
            if uses != UPLOAD_ACTION {
                bail!("acceptance producers: {path} must use the pinned receipt upload action");
            }
            if optional(step, "continue-on-error").is_some() {
                bail!("acceptance producers: {path} may not declare continue-on-error");
            }
            let condition = string(required(step, "if", &path)?, &format!("{path}.if"))?;
            if condition != "${{ always() }}" {
                bail!("acceptance producers: {path}.if must equal ${{{{ always() }}}}");
            }
            if let Some((_, expected_path)) = EXPECTED
                .iter()
                .find(|(expected_name, _)| *expected_name == name)
            {
                let upload_path = string(
                    required(with, "path", &format!("{path}.with"))?,
                    &format!("{path}.with.path"),
                )?;
                if upload_path != *expected_path {
                    bail!("acceptance producers: {path}.with.path must equal {expected_path}");
                }
                let missing = string(
                    required(with, "if-no-files-found", &format!("{path}.with"))?,
                    &format!("{path}.with.if-no-files-found"),
                )?;
                if missing != "error" {
                    bail!("acceptance producers: {path}.with.if-no-files-found must equal error");
                }
            }
            *actual.entry(name.to_owned()).or_default() += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    fn workflow(names: &[&str]) -> String {
        let steps = names
            .iter()
            .map(|name| {
                let path = super::EXPECTED
                    .iter()
                    .find(|(expected, _)| expected == name)
                    .unwrap()
                    .1;
                format!(
                    "      - if: ${{{{ always() }}}}\n        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a\n        with:\n          name: {name}\n          path: {path}\n          if-no-files-found: error\n"
                )
            })
            .collect::<String>();
        format!("jobs:\n  receipts:\n    steps:\n{steps}")
    }

    #[test]
    fn receipt_upload_set_is_exact_and_non_tolerated() {
        let candidate = workflow(&[
            "deve-acceptance-receipts-docker-${{ github.sha }}",
            "deve-acceptance-receipts-contracts-${{ github.sha }}",
            "deve-acceptance-receipts-release-${{ github.sha }}",
            "deve-acceptance-receipts-repo-linux-${{ github.sha }}",
            "deve-acceptance-receipts-security-${{ github.sha }}",
            "deve-acceptance-receipts-github-${{ github.sha }}",
        ]);
        let native = workflow(&[
            "deve-acceptance-receipts-desktop-local-${{ inputs.candidate_head }}",
            "deve-acceptance-receipts-desktop-remote-${{ inputs.candidate_head }}",
            "deve-acceptance-receipts-desktop-macos-${{ inputs.candidate_head }}",
            "deve-acceptance-receipts-android-local-${{ inputs.candidate_head }}",
            "deve-acceptance-receipts-android-remote-${{ inputs.candidate_head }}",
        ]);
        validate(&candidate, &native).unwrap();
        assert!(
            validate(
                &candidate.replace("receipts-docker", "receipts-docker-renamed"),
                &native
            )
            .is_err()
        );
        assert!(
            validate(
                &candidate.replace(
                    "${{ runner.temp }}/deve-acceptance-contracts",
                    "${{ runner.temp }}/wrong"
                ),
                &native
            )
            .is_err()
        );
        assert!(
            validate(
                &candidate.replace("if-no-files-found: error", "if-no-files-found: warn"),
                &native
            )
            .is_err()
        );
        assert!(
            validate(
                &candidate,
                &native.replace(
                    "      - if: ${{ always() }}",
                    "      - if: ${{ always() }}\n        continue-on-error: true"
                )
            )
            .is_err()
        );
    }
}
