//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 05_diff_logic#source-control-runtime

use crate::models::DocId;
use crate::source_control::ChangeDomain;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScPathTarget {
    pub path: String,
    #[serde(default)]
    pub doc_id: Option<DocId>,
    #[serde(default)]
    pub domain: Option<ChangeDomain>,
}

impl ScPathTarget {
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            doc_id: None,
            domain: None,
        }
    }
}
