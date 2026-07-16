//! Acceptance receipt and command execution data model.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::acceptance_matrix) struct Receipt {
    pub(in crate::acceptance_matrix) schema: u8,
    pub(in crate::acceptance_matrix) producer_id: String,
    pub(in crate::acceptance_matrix) producer_contract: String,
    pub(in crate::acceptance_matrix) execution_id: String,
    pub(in crate::acceptance_matrix) execution_evidence_ids: Vec<String>,
    pub(in crate::acceptance_matrix) evidence_id: String,
    pub(in crate::acceptance_matrix) evidence_ref: String,
    pub(in crate::acceptance_matrix) head: String,
    pub(in crate::acceptance_matrix) head_after: Option<String>,
    pub(in crate::acceptance_matrix) dirty_before: bool,
    pub(in crate::acceptance_matrix) dirty_after: bool,
    pub(in crate::acceptance_matrix) os: String,
    pub(in crate::acceptance_matrix) arch: String,
    pub(in crate::acceptance_matrix) target_os: String,
    pub(in crate::acceptance_matrix) surface: String,
    pub(in crate::acceptance_matrix) mode: String,
    pub(in crate::acceptance_matrix) started_at: String,
    pub(in crate::acceptance_matrix) finished_at: String,
    pub(in crate::acceptance_matrix) status: String,
    pub(in crate::acceptance_matrix) exit_code: Option<i32>,
    pub(in crate::acceptance_matrix) error: Option<String>,
    pub(in crate::acceptance_matrix) command_program: String,
    pub(in crate::acceptance_matrix) command_arg_count: usize,
    pub(in crate::acceptance_matrix) command_fingerprint: String,
    pub(in crate::acceptance_matrix) command_artifacts: Vec<String>,
    #[serde(default)]
    pub(in crate::acceptance_matrix) producer_inputs: BTreeMap<String, String>,
    pub(in crate::acceptance_matrix) claims: Option<Value>,
}

#[derive(Clone, Debug)]
pub(in crate::acceptance_matrix) struct EvidenceSpec {
    pub(in crate::acceptance_matrix) evidence_id: String,
    pub(in crate::acceptance_matrix) evidence_ref: String,
    pub(in crate::acceptance_matrix) surface: String,
    pub(in crate::acceptance_matrix) mode: String,
    pub(in crate::acceptance_matrix) target_os: String,
    pub(in crate::acceptance_matrix) output: PathBuf,
    pub(in crate::acceptance_matrix) claims: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub(in crate::acceptance_matrix) struct CommandStep {
    pub(in crate::acceptance_matrix) program: String,
    pub(in crate::acceptance_matrix) args: Vec<String>,
    pub(in crate::acceptance_matrix) env: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub(in crate::acceptance_matrix) struct ExecutionSpec {
    pub(in crate::acceptance_matrix) producer_id: String,
    pub(in crate::acceptance_matrix) producer_contract: String,
    pub(in crate::acceptance_matrix) command_artifacts: Vec<String>,
    pub(in crate::acceptance_matrix) producer_inputs: BTreeMap<String, String>,
    pub(in crate::acceptance_matrix) steps: Vec<CommandStep>,
    pub(in crate::acceptance_matrix) finally_steps: Vec<CommandStep>,
    pub(in crate::acceptance_matrix) timeout: Duration,
}
