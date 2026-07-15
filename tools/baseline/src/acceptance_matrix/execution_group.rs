//! Cross-artifact receipt execution-group invariants.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use super::receipt::Receipt;
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_execution_groups<'a>(
    receipts: impl IntoIterator<Item = &'a Receipt>,
    label: &str,
) -> Result<()> {
    let receipts = receipts.into_iter().collect::<Vec<_>>();
    let mut evidence = BTreeSet::new();
    let mut groups = BTreeMap::<&str, Vec<&Receipt>>::new();
    for receipt in receipts {
        if receipt.producer_id.trim().is_empty()
            || receipt.producer_contract.trim().is_empty()
            || receipt.execution_id.trim().is_empty()
        {
            bail!("{label}: receipt producer/execution binding is missing");
        }
        if !evidence.insert(receipt.evidence_id.as_str()) {
            bail!(
                "{label}: duplicate receipt evidence_id {}",
                receipt.evidence_id
            );
        }
        groups
            .entry(receipt.execution_id.as_str())
            .or_default()
            .push(receipt);
    }

    for (execution_id, group) in groups {
        let first = group[0];
        let expected = evidence_set(first, label)?;
        let observed = group
            .iter()
            .map(|receipt| receipt.evidence_id.as_str())
            .collect::<BTreeSet<_>>();
        if observed != expected {
            bail!(
                "{label}: execution {execution_id} is missing evidence or contains undeclared evidence"
            );
        }
        for receipt in group {
            if evidence_set(receipt, label)? != expected || !same_execution_context(first, receipt)
            {
                bail!("{label}: mixed or inconsistent receipt execution group {execution_id}");
            }
        }
    }
    Ok(())
}

fn evidence_set<'a>(receipt: &'a Receipt, label: &str) -> Result<BTreeSet<&'a str>> {
    let expected = receipt
        .execution_evidence_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if expected.len() != receipt.execution_evidence_ids.len()
        || !expected.contains(receipt.evidence_id.as_str())
    {
        bail!("{label}: receipt execution evidence set is invalid");
    }
    Ok(expected)
}

fn same_execution_context(left: &Receipt, right: &Receipt) -> bool {
    left.producer_id == right.producer_id
        && left.producer_contract == right.producer_contract
        && left.execution_id == right.execution_id
        && left.head == right.head
        && left.head_after == right.head_after
        && left.dirty_before == right.dirty_before
        && left.dirty_after == right.dirty_after
        && left.os == right.os
        && left.arch == right.arch
        && left.started_at == right.started_at
        && left.finished_at == right.finished_at
        && left.status == right.status
        && left.exit_code == right.exit_code
        && left.command_program == right.command_program
        && left.command_arg_count == right.command_arg_count
        && left.command_fingerprint == right.command_fingerprint
        && left.command_artifacts == right.command_artifacts
        && left.producer_inputs == right.producer_inputs
}

#[cfg(test)]
mod tests {
    use super::validate_execution_groups;
    use crate::acceptance_matrix::receipt::Receipt;
    use std::collections::BTreeMap;

    fn receipt(evidence_id: &str) -> Receipt {
        Receipt {
            schema: 3,
            producer_id: "producer.group".into(),
            producer_contract: "fnv1a64:1111111111111111".into(),
            execution_id: "exec-fnv1a64-2222222222222222".into(),
            execution_evidence_ids: vec!["smoke.one".into(), "smoke.two".into()],
            evidence_id: evidence_id.into(),
            evidence_ref: format!("receipts/{evidence_id}.json"),
            head: "abc".into(),
            head_after: Some("abc".into()),
            dirty_before: false,
            dirty_after: false,
            os: "linux".into(),
            arch: "x86_64".into(),
            target_os: "web".into(),
            surface: "web".into(),
            mode: "browser".into(),
            started_at: "2026-01-01T00:00:00Z".into(),
            finished_at: "2026-01-01T00:00:01Z".into(),
            status: "passed".into(),
            exit_code: Some(0),
            error: None,
            command_program: "test".into(),
            command_arg_count: 1,
            command_fingerprint: "fnv1a64:3333333333333333".into(),
            command_artifacts: vec!["scripts/test.sh".into()],
            producer_inputs: BTreeMap::from([("DEVE_IMAGE_ID".into(), "sha256:test".into())]),
            claims: None,
        }
    }

    #[test]
    fn complete_execution_group_accepts_evidence_specific_projection_fields() {
        let one = receipt("smoke.one");
        let mut two = receipt("smoke.two");
        two.surface = "docker".into();
        two.mode = "multiclient".into();
        two.target_os = "linux".into();

        validate_execution_groups([&one, &two], "fixture").unwrap();
    }

    #[test]
    fn execution_group_rejects_inconsistent_common_fields() {
        let one = receipt("smoke.one");
        let mut cases = Vec::new();
        let mut fingerprint = receipt("smoke.two");
        fingerprint.command_fingerprint = "fnv1a64:4444444444444444".into();
        cases.push(fingerprint);
        let mut inputs = receipt("smoke.two");
        inputs
            .producer_inputs
            .insert("DEVE_IMAGE_ID".into(), "sha256:other".into());
        cases.push(inputs);
        let mut head = receipt("smoke.two");
        head.head = "def".into();
        cases.push(head);
        let mut status = receipt("smoke.two");
        status.status = "failed".into();
        cases.push(status);
        let mut producer = receipt("smoke.two");
        producer.producer_id = "producer.other".into();
        cases.push(producer);

        for sibling in &cases {
            assert!(validate_execution_groups([&one, sibling], "fixture").is_err());
        }
    }
}
