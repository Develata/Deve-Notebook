//! Typed shadow-only CI impact registry and plan model.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) const IMPACT_REGISTRY_PATH: &str = "docs/registry/acceptance-impact.json";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImpactRegistry {
    pub(super) schema: u8,
    pub(super) mode: String,
    pub(super) artifacts: Vec<ArtifactInput>,
    pub(super) profiles: Vec<ImpactProfile>,
    pub(super) modules: Vec<ImpactModule>,
    pub(super) shards: Vec<ImpactShard>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactInput {
    pub(super) artifact_id: String,
    pub(super) identity: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImpactProfile {
    pub(super) profile_id: String,
    pub(super) selection: String,
    pub(super) always_shards: Vec<String>,
    pub(super) shards: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImpactModule {
    pub(super) module_id: String,
    pub(super) dependencies: Vec<String>,
    pub(super) full_trigger: bool,
    pub(super) exact_paths: Vec<String>,
    pub(super) path_prefixes: Vec<String>,
    pub(super) shards: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImpactShard {
    pub(super) shard_id: String,
    pub(super) layer: String,
    pub(super) execution_kind: String,
    pub(super) producer_ids: Vec<String>,
    pub(super) checks: Vec<String>,
    pub(super) artifact_inputs: Vec<String>,
    pub(super) isolation: Isolation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Isolation {
    pub(super) state: String,
    pub(super) ports: String,
    pub(super) database: String,
    pub(super) identity: String,
    pub(super) fixture: String,
    pub(super) logs: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ImpactPlan {
    pub(super) schema: u8,
    pub(super) status: &'static str,
    pub(super) input_fingerprints: InputFingerprints,
    pub(super) profile: String,
    pub(super) selection: String,
    pub(super) scope: String,
    pub(super) base: Option<String>,
    pub(super) head: Option<String>,
    pub(super) changed_files: Vec<String>,
    pub(super) full_reasons: Vec<String>,
    pub(super) changed_modules: Vec<String>,
    pub(super) reverse_consumers: Vec<String>,
    pub(super) selected_modules: Vec<String>,
    pub(super) selected_shards: Vec<String>,
    pub(super) producer_ids: Vec<String>,
    pub(super) evidence_ids: Vec<String>,
    pub(super) case_ids: Vec<String>,
    pub(super) checks: Vec<String>,
    pub(super) artifact_inputs: Vec<String>,
    pub(super) isolation: BTreeMap<String, Isolation>,
}

#[derive(Debug, Serialize)]
pub(super) struct InputFingerprints {
    pub(super) impact_registry: String,
    pub(super) producer_registry: String,
    pub(super) acceptance_matrix: String,
}
