//! Candidate-to-native reachability and contract-receipt delivery-chain validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::{mapping, optional, required, sequence, string};
use anyhow::{Context, Result, bail};
use yaml_rust2::{Yaml, YamlLoader};

const NATIVE_WORKFLOW: &str = "./.github/workflows/release-native.yml";
const UPLOAD_ACTION: &str = "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a";
const DOWNLOAD_ACTION: &str = "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c";

pub(super) fn validate_candidate(content: &str) -> Result<()> {
    let documents = YamlLoader::load_from_str(content)
        .context("acceptance producers: release-candidate.yml is not valid YAML")?;
    let [document] = documents.as_slice() else {
        bail!("acceptance producers: release-candidate.yml must contain one YAML document");
    };
    let root = mapping(document, "release-candidate.yml")?;
    if optional(root, "defaults").is_some() {
        bail!("acceptance producers: candidate workflow may not override run defaults");
    }
    let jobs = mapping(
        required(root, "jobs", "release-candidate.yml")?,
        "release-candidate.yml.jobs",
    )?;
    validate_native_call(jobs)?;
    validate_contract_delivery(jobs)
}

fn validate_native_call(jobs: &yaml_rust2::yaml::Hash) -> Result<()> {
    let mut calls = 0usize;
    for (job_key, job_value) in jobs {
        let job_id = string(job_key, "release-candidate.yml.jobs key")?;
        let path = format!("release-candidate.yml.jobs.{job_id}");
        let job = mapping(job_value, &path)?;
        let Some(uses) = optional(job, "uses") else {
            continue;
        };
        if string(uses, &format!("{path}.uses"))? != NATIVE_WORKFLOW {
            continue;
        }
        calls += 1;
        reject_execution_modifiers(job, &path)?;
        require_needs(job, &["identity", "web-dist"], &path)?;
        let with = mapping(required(job, "with", &path)?, &format!("{path}.with"))?;
        require_string(with, "candidate_head", "${{ github.sha }}", &path)?;
        require_string(with, "version", "${{ inputs.version }}", &path)?;
        require_string(
            with,
            "web_dist_artifact",
            "deve-candidate-web-dist-${{ github.sha }}",
            &path,
        )?;
        validate_native_secrets(job, &path)?;
    }
    if calls != 1 {
        bail!(
            "acceptance producers: release candidate must call release-native.yml exactly once, found {calls}"
        );
    }
    Ok(())
}

fn validate_native_secrets(job: &yaml_rust2::yaml::Hash, path: &str) -> Result<()> {
    let secrets = mapping(required(job, "secrets", path)?, &format!("{path}.secrets"))?;
    let expected = [
        (
            "ANDROID_KEYSTORE_BASE64",
            "${{ secrets.ANDROID_KEYSTORE_BASE64 }}",
        ),
        (
            "ANDROID_KEYSTORE_PASSWORD",
            "${{ secrets.ANDROID_KEYSTORE_PASSWORD }}",
        ),
        ("ANDROID_KEY_ALIAS", "${{ secrets.ANDROID_KEY_ALIAS }}"),
        (
            "ANDROID_KEY_PASSWORD",
            "${{ secrets.ANDROID_KEY_PASSWORD }}",
        ),
    ];
    if secrets.len() != expected.len() {
        bail!("acceptance producers: {path}.secrets must use the exact Android signing set");
    }
    for (key, value) in expected {
        require_string(secrets, key, value, &format!("{path}.secrets"))?;
    }
    Ok(())
}

fn validate_contract_delivery(jobs: &yaml_rust2::yaml::Hash) -> Result<()> {
    let contract = mapping(
        required(jobs, "contract-receipts", "release-candidate.yml.jobs")?,
        "release-candidate.yml.jobs.contract-receipts",
    )?;
    reject_execution_modifiers(contract, "release-candidate.yml.jobs.contract-receipts")?;
    require_needs(
        contract,
        &["identity"],
        "release-candidate.yml.jobs.contract-receipts",
    )?;
    let steps = sequence(
        required(
            contract,
            "steps",
            "release-candidate.yml.jobs.contract-receipts",
        )?,
        "release-candidate.yml.jobs.contract-receipts.steps",
    )?;
    let mut uploads = 0usize;
    for (index, step_value) in steps.iter().enumerate() {
        let path = format!("release-candidate.yml.jobs.contract-receipts.steps[{index}]");
        let step = mapping(step_value, &path)?;
        let Some(uses) = optional(step, "uses") else {
            continue;
        };
        if string(uses, &format!("{path}.uses"))? != UPLOAD_ACTION {
            continue;
        }
        uploads += 1;
        if optional(step, "continue-on-error").is_some() {
            bail!("acceptance producers: {path} may not declare continue-on-error");
        }
        require_string(step, "if", "${{ always() }}", &path)?;
        let with = mapping(required(step, "with", &path)?, &format!("{path}.with"))?;
        require_string(
            with,
            "name",
            "deve-acceptance-receipts-contracts-${{ github.sha }}",
            &path,
        )?;
        require_string(
            with,
            "path",
            "${{ runner.temp }}/deve-acceptance-contracts",
            &path,
        )?;
        require_string(with, "if-no-files-found", "error", &path)?;
    }
    if uploads != 1 {
        bail!("acceptance producers: contract-receipts must upload its receipt root exactly once");
    }
    let assemble = mapping(
        required(jobs, "assemble", "release-candidate.yml.jobs")?,
        "release-candidate.yml.jobs.assemble",
    )?;
    reject_execution_modifiers(assemble, "release-candidate.yml.jobs.assemble")?;
    require_needs(
        assemble,
        &[
            "identity",
            "contract-static",
            "rust-quality",
            "workspace-tests",
            "full-baseline",
            "web-dist",
            "docker-linux-amd64-build",
            "docker-linux-amd64-smoke",
            "contract-receipts",
            "repo-process-linux",
            "security-receipts",
            "github-receipts",
            "native",
        ],
        "release-candidate.yml.jobs.assemble",
    )?;
    validate_receipt_download(assemble)
}

fn validate_receipt_download(assemble: &yaml_rust2::yaml::Hash) -> Result<()> {
    let job_path = "release-candidate.yml.jobs.assemble";
    let steps = sequence(
        required(assemble, "steps", job_path)?,
        "release-candidate.yml.jobs.assemble.steps",
    )?;
    let mut receipt_downloads = 0usize;
    for (index, step_value) in steps.iter().enumerate() {
        let path = format!("release-candidate.yml.jobs.assemble.steps[{index}]");
        let step = mapping(step_value, &path)?;
        let Some(uses) = optional(step, "uses") else {
            continue;
        };
        if string(uses, &format!("{path}.uses"))? != DOWNLOAD_ACTION {
            continue;
        }
        let with = mapping(required(step, "with", &path)?, &format!("{path}.with"))?;
        if optional(with, "pattern").is_none() {
            continue;
        }
        receipt_downloads += 1;
        for key in ["if", "continue-on-error"] {
            if optional(step, key).is_some() {
                bail!("acceptance producers: {path} may not declare {key}");
            }
        }
        if with.len() != 3 {
            bail!(
                "acceptance producers: {path}.with must contain only pattern, path, and merge-multiple"
            );
        }
        require_string(
            with,
            "pattern",
            "deve-acceptance-receipts-*-${{ github.sha }}",
            &path,
        )?;
        require_string(
            with,
            "path",
            "${{ runner.temp }}/deve-release-incoming/receipts",
            &path,
        )?;
        if !matches!(optional(with, "merge-multiple"), Some(Yaml::Boolean(false))) {
            bail!("acceptance producers: {path}.merge-multiple must be false");
        }
    }
    if receipt_downloads != 1 {
        bail!("acceptance producers: assemble must download the receipt artifact set exactly once");
    }
    Ok(())
}

fn reject_execution_modifiers(mapping: &yaml_rust2::yaml::Hash, path: &str) -> Result<()> {
    for key in ["if", "continue-on-error", "strategy", "defaults"] {
        if optional(mapping, key).is_some() {
            bail!("acceptance producers: {path} may not declare {key}");
        }
    }
    Ok(())
}

fn require_string(
    mapping: &yaml_rust2::yaml::Hash,
    key: &str,
    expected: &str,
    path: &str,
) -> Result<()> {
    let actual = string(required(mapping, key, path)?, &format!("{path}.{key}"))?;
    if actual != expected {
        bail!("acceptance producers: {path}.{key} must equal {expected}");
    }
    Ok(())
}

fn require_needs(job: &yaml_rust2::yaml::Hash, required_needs: &[&str], path: &str) -> Result<()> {
    let raw = required(job, "needs", path)?;
    let needs = match raw {
        Yaml::String(value) => vec![value.as_str()],
        Yaml::Array(values) => values
            .iter()
            .map(|value| string(value, &format!("{path}.needs")))
            .collect::<Result<Vec<_>>>()?,
        _ => bail!("acceptance producers: {path}.needs must be a string or sequence"),
    };
    for required_need in required_needs {
        if !needs.contains(required_need) {
            bail!("acceptance producers: {path}.needs must include {required_need}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_candidate;

    const VALID: &str = r#"
jobs:
  native:
    needs: [identity, web-dist]
    uses: ./.github/workflows/release-native.yml
    with:
      candidate_head: ${{ github.sha }}
      version: ${{ inputs.version }}
      web_dist_artifact: deve-candidate-web-dist-${{ github.sha }}
    secrets:
      ANDROID_KEYSTORE_BASE64: ${{ secrets.ANDROID_KEYSTORE_BASE64 }}
      ANDROID_KEYSTORE_PASSWORD: ${{ secrets.ANDROID_KEYSTORE_PASSWORD }}
      ANDROID_KEY_ALIAS: ${{ secrets.ANDROID_KEY_ALIAS }}
      ANDROID_KEY_PASSWORD: ${{ secrets.ANDROID_KEY_PASSWORD }}
  contract-receipts:
    needs: identity
    steps:
      - if: ${{ always() }}
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
        with:
          name: deve-acceptance-receipts-contracts-${{ github.sha }}
          path: ${{ runner.temp }}/deve-acceptance-contracts
          if-no-files-found: error
  assemble:
    needs: [identity, contract-static, rust-quality, workspace-tests, full-baseline, web-dist, docker-linux-amd64-build, docker-linux-amd64-smoke, contract-receipts, repo-process-linux, security-receipts, github-receipts, native]
    steps:
      - uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c
        with:
          pattern: deve-acceptance-receipts-*-${{ github.sha }}
          path: ${{ runner.temp }}/deve-release-incoming/receipts
          merge-multiple: false
"#;

    #[test]
    fn native_call_and_receipt_delivery_are_required_execution_edges() {
        validate_candidate(VALID).unwrap();
        assert!(
            validate_candidate(&VALID.replace(
                "    uses: ./.github/workflows/release-native.yml",
                "    uses: ./.github/workflows/other.yml"
            ))
            .is_err()
        );
        assert!(
            validate_candidate(&VALID.replace("  native:\n", "  native:\n    if: false\n"))
                .is_err()
        );
        assert!(
            validate_candidate(&VALID.replace(
                "needs: [identity, contract-static, rust-quality, workspace-tests, full-baseline, web-dist, docker-linux-amd64-build, docker-linux-amd64-smoke, contract-receipts, repo-process-linux, security-receipts, github-receipts, native]",
                "needs: [identity, contract-static, rust-quality, workspace-tests, full-baseline, web-dist, docker-linux-amd64-build, docker-linux-amd64-smoke, repo-process-linux, security-receipts, github-receipts, native]"
            ))
            .is_err()
        );
        assert!(
            validate_candidate(
                &VALID.replace("if-no-files-found: error", "if-no-files-found: warn")
            )
            .is_err()
        );
        assert!(
            validate_candidate(&VALID.replace(
                "uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
                "continue-on-error: true\n        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"
            ))
            .is_err()
        );
        assert!(
            validate_candidate(&VALID.replace(
                "deve-acceptance-receipts-*-${{ github.sha }}",
                "deve-acceptance-receipts-native-${{ github.sha }}"
            ))
            .is_err()
        );
        assert!(
            validate_candidate(&VALID.replace("  assemble:\n", "  assemble:\n    if: false\n"))
                .is_err()
        );
        assert!(
            validate_candidate(&VALID.replace(
                "      - uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c",
                "      - if: false\n        uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c"
            ))
            .is_err()
        );
        assert!(
            validate_candidate(&VALID.replace(
                "          merge-multiple: false",
                "          merge-multiple: false\n          run-id: 1"
            ))
            .is_err()
        );
    }
}
