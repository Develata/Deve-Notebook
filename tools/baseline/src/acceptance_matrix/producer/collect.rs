//! Safe cross-platform acceptance receipt aggregation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::artifact_reader::{ReceiptArtifactBudget, ReceiptArtifactRoot};
use crate::acceptance_matrix::execution_group::validate_execution_groups;
use crate::acceptance_matrix::receipt::{Receipt, ensure_output_outside_worktree};
use crate::context::BaselineContext;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn run(args: &[String]) -> Result<()> {
    let parsed = CollectArgs::parse(args)?;
    let ctx = BaselineContext::new("acceptance-collect")?;
    let output = absolute(ctx.root(), &parsed.output);
    ensure_output_outside_worktree(ctx.root(), &output)?;
    if output.exists() {
        bail!(
            "acceptance-collect: output already exists: {}",
            output.display()
        );
    }
    let parent = output
        .parent()
        .context("acceptance-collect: output has no parent")?;
    fs::create_dir_all(parent)?;
    let name = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("receipts");
    let staging = parent.join(format!(".{name}.collect-{}", std::process::id()));
    if staging.exists() {
        bail!(
            "acceptance-collect: staging path already exists: {}",
            staging.display()
        );
    }
    fs::create_dir(&staging)?;

    let result = collect_into(ctx.root(), &parsed.inputs, &staging);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if let Err(error) = ensure_output_outside_worktree(ctx.root(), &output) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if output.exists() {
        let _ = fs::remove_dir_all(&staging);
        bail!(
            "acceptance-collect: output appeared during collection: {}",
            output.display()
        );
    }
    fs::rename(&staging, &output)
        .with_context(|| format!("acceptance-collect: failed to publish {}", output.display()))?;
    println!("acceptance-collect: receipts -> {}", output.display());
    Ok(())
}

fn collect_into(root: &Path, inputs: &[PathBuf], staging: &Path) -> Result<()> {
    let mut evidence = BTreeMap::<String, String>::new();
    let mut locators = BTreeMap::<String, PathBuf>::new();
    let mut inventory = Vec::new();
    let mut budget = ReceiptArtifactBudget::default();
    for input in inputs {
        let input = absolute(root, input);
        let reader = ReceiptArtifactRoot::open(&input)?;
        for (relative, source) in reader.json_files()? {
            let content = reader.read_json(&source, &mut budget)?;
            let receipt: Receipt = serde_json::from_slice(&content)
                .with_context(|| format!("invalid acceptance receipt {}", source.display()))?;
            if receipt.schema != 3 {
                bail!("acceptance-collect: only schema 3 receipts are accepted");
            }
            if relative != receipt.evidence_ref {
                bail!(
                    "acceptance-collect: locator {} does not match receipt {}",
                    relative,
                    receipt.evidence_ref
                );
            }
            if let Some(previous) = evidence.insert(receipt.evidence_id.clone(), relative.clone()) {
                bail!(
                    "acceptance-collect: duplicate evidence_id {} at {} and {}",
                    receipt.evidence_id,
                    previous,
                    relative
                );
            }
            if let Some(previous) = locators.insert(relative.clone(), source.clone()) {
                bail!(
                    "acceptance-collect: duplicate locator {} from {} and {}",
                    relative,
                    previous.display(),
                    source.display()
                );
            }
            let target = staging.join(Path::new(&relative));
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, content)?;
            inventory.push((relative, receipt));
        }
    }
    if evidence.is_empty() {
        bail!("acceptance-collect: inputs contain no receipt JSON files");
    }
    validate_execution_groups(
        inventory.iter().map(|(_, receipt)| receipt),
        "acceptance-collect",
    )?;
    Ok(())
}

fn absolute(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

struct CollectArgs {
    output: PathBuf,
    inputs: Vec<PathBuf>,
}

impl CollectArgs {
    fn parse(args: &[String]) -> Result<Self> {
        let Some(output_index) = args.iter().position(|arg| arg == "--output") else {
            bail!("acceptance-collect: --output <receipt-root> is required");
        };
        let output = args
            .get(output_index + 1)
            .context("acceptance-collect: --output requires a value")?;
        let mut inputs = Vec::new();
        let mut index = 0usize;
        while index < args.len() {
            if index == output_index {
                index += 2;
            } else if args[index].starts_with('-') {
                bail!("acceptance-collect: unknown option {}", args[index]);
            } else {
                inputs.push(PathBuf::from(&args[index]));
                index += 1;
            }
        }
        if inputs.is_empty() {
            bail!("acceptance-collect: at least one artifact root is required");
        }
        Ok(Self {
            output: PathBuf::from(output),
            inputs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CollectArgs, collect_into};
    use crate::acceptance_matrix::receipt::Receipt;
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn collect_args_require_output_and_inputs() {
        let args = ["--output", "out", "one", "two"].map(str::to_string);
        let parsed = CollectArgs::parse(&args).unwrap();
        assert_eq!(parsed.inputs.len(), 2);
        assert_eq!(parsed.output.to_string_lossy(), "out");
    }

    fn temp_root() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let serial = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "deve-acceptance-collect-{}-{unique}-{serial}",
            std::process::id()
        ))
    }

    fn write_receipt(root: &std::path::Path, id: &str, locator: &str) {
        let path = root.join(locator);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let receipt = Receipt {
            schema: 3,
            producer_id: format!("producer.{id}"),
            producer_contract: "fnv1a64:1111111111111111".into(),
            execution_id: format!("exec-fnv1a64-{id:0<16}"),
            execution_evidence_ids: vec![id.into()],
            evidence_id: id.into(),
            evidence_ref: locator.into(),
            head: "abc".into(),
            head_after: Some("abc".into()),
            dirty_before: false,
            dirty_after: false,
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            target_os: "web".into(),
            surface: "web".into(),
            mode: "browser".into(),
            started_at: "2026-01-01T00:00:00Z".into(),
            finished_at: "2026-01-01T00:00:01Z".into(),
            status: "passed".into(),
            exit_code: Some(0),
            error: None,
            command_program: "test".into(),
            command_arg_count: 0,
            command_fingerprint: "fnv1a64:0123456789abcdef".into(),
            command_artifacts: Vec::new(),
            producer_inputs: BTreeMap::new(),
            claims: None,
        };
        fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }

    #[test]
    fn collector_rejects_duplicate_evidence_before_publication() {
        let base = temp_root();
        let one = base.join("one");
        let two = base.join("two");
        let staging = base.join("staging");
        fs::create_dir_all(&staging).unwrap();
        write_receipt(&one, "smoke.same", "receipts/one.json");
        write_receipt(&two, "smoke.same", "receipts/two.json");
        let error = collect_into(&base, &[one, two], &staging).unwrap_err();
        assert!(error.to_string().contains("duplicate evidence_id"));
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn collector_rejects_partial_execution_group() {
        let base = temp_root();
        let input = base.join("input");
        let staging = base.join("staging");
        fs::create_dir_all(&staging).unwrap();
        write_receipt(&input, "smoke.one", "receipts/one.json");
        let path = input.join("receipts/one.json");
        let mut receipt: Receipt = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        receipt.execution_evidence_ids.push("smoke.two".into());
        fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        let error = collect_into(&base, &[input], &staging).unwrap_err();
        assert!(error.to_string().contains("missing evidence"));
        let _ = fs::remove_dir_all(base);
    }
}
