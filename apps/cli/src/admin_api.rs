use deve_core::models::{DocId, LedgerEntry, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpResponse {
    pub doc_id: DocId,
    pub ops: Vec<(u64, LedgerEntry)>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEntry {
    pub global_seq: u64,
    pub current_path: Option<String>,
    pub entry: LedgerEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCheckResponse {
    pub repo_name: String,
    pub missing_nodes: Vec<(DocId, String)>,
    pub orphan_nodes: Vec<(NodeId, String)>,
}
