//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent

use serde::{Deserialize, Serialize};

/// Structured merge conflict hunk shared by server and WASM protocol consumers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictHunk {
    pub start_line: usize,
    pub length: usize,
    pub local_lines: Vec<String>,
    pub remote_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeConflictAction {
    AcceptCurrent,
    AcceptIncoming,
    AcceptBoth,
}
