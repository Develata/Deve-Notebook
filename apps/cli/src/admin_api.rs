//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 03_storage/repair#backup-export
//!   - 04_repository#tree-projection-contract
//!   - 05_diff_logic#source-control-runtime
//!   - 14_commands#cli-commands

use deve_core::models::{DocId, LedgerEntry, NodeId, NodeMeta};
use deve_core::source_control::ChangeEntry;
use deve_core::sync::{ProjectionDiagnostic, ProjectionDiagnosticStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpResponse {
    pub doc_id: Option<DocId>,
    pub node_id: Option<NodeId>,
    pub node_meta: Option<NodeMeta>,
    pub ops: Vec<(u64, LedgerEntry)>,
    pub structure_ops: Vec<(u64, LedgerEntry)>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionCheckResponse {
    pub repo_name: String,
    pub status: String,
    pub issue_code: Option<String>,
    pub issue_detail: Option<String>,
    pub rebuild_supported: bool,
    #[serde(default)]
    pub repair_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScStatusResponse {
    pub repo_name: String,
    pub staged: Vec<ChangeEntry>,
    pub unstaged: Vec<ChangeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusResponse {
    pub repo_name: String,
    pub status: deve_core::git_bridge::GitMirrorStatus,
    pub summary: deve_core::git_bridge::GitMirrorSummary,
    pub records: Vec<deve_core::git_bridge::GitMirrorRecord>,
}

impl ProjectionCheckResponse {
    pub fn from_diagnostic(diagnostic: ProjectionDiagnostic) -> Self {
        let (issue_code, issue_detail) = diagnostic
            .issue
            .map(|issue| (Some(issue.code), Some(issue.detail)))
            .unwrap_or((None, None));
        Self {
            repo_name: diagnostic.repo_name,
            status: projection_status_text(diagnostic.status).to_string(),
            issue_code,
            issue_detail,
            rebuild_supported: diagnostic.rebuild_supported,
            repair_hint: diagnostic.repair_hint.to_string(),
        }
    }
}

fn projection_status_text(status: ProjectionDiagnosticStatus) -> &'static str {
    match status {
        ProjectionDiagnosticStatus::Healthy => "healthy",
        ProjectionDiagnosticStatus::AuthorityCorrupt => "authority_corrupt",
    }
}
