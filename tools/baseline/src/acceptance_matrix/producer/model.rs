//! Typed acceptance producer registry model.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) const PRODUCER_REGISTRY_PATH: &str = "docs/registry/acceptance-producers.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProducerRegistry {
    pub(super) schema: u8,
    pub(super) producers: Vec<Producer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Producer {
    pub(super) producer_id: String,
    pub(super) evidence_ids: Vec<String>,
    #[serde(default)]
    pub(super) dependencies: Vec<String>,
    pub(super) tiers: Vec<String>,
    pub(super) host_os: Vec<String>,
    pub(super) timeout_seconds: u64,
    #[serde(default)]
    pub(super) required_env: Vec<String>,
    /// Required environment values that are safe to publish in receipts and
    /// form part of the evidence identity (for example an immutable image ID).
    #[serde(default)]
    pub(super) bound_env: Vec<String>,
    #[serde(default)]
    pub(super) environment: BTreeMap<String, String>,
    #[serde(default)]
    pub(super) claims_env: BTreeMap<String, String>,
    pub(super) artifacts: Vec<String>,
    pub(super) steps: Vec<ProducerStep>,
    #[serde(default)]
    pub(super) finally_steps: Vec<ProducerStep>,
    pub(super) note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProducerStep {
    pub(super) program: String,
    #[serde(default)]
    pub(super) args: Vec<ProducerArg>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(super) enum ProducerArg {
    LiteralString(String),
    Literal { literal: String },
    Env { env: String },
}
