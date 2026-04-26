use serde::{Deserialize, Serialize};

/// Structured merge conflict hunk shared by server and WASM protocol consumers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictHunk {
    pub start_line: usize,
    pub length: usize,
    pub local_lines: Vec<String>,
    pub remote_lines: Vec<String>,
}
