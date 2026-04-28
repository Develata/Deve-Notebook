//! plan_ref:
//!   - 04_storage#internal-path-normalization
//!   - 07_diff_logic#source-control-runtime

use crate::models::DocId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScPathTarget {
    pub path: String,
    #[serde(default)]
    pub doc_id: Option<DocId>,
}

impl ScPathTarget {
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            doc_id: None,
        }
    }
}
