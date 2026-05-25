//! plan_ref:
//!   - 04_repository#tree-projection-contract

use super::{materialize, projection_plan};
use crate::ledger::RepoManager;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDiagnostic {
    pub repo_name: String,
    pub status: ProjectionDiagnosticStatus,
    pub issue: Option<ProjectionDiagnosticIssue>,
    pub rebuild_supported: bool,
    pub repair_hint: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionDiagnosticStatus {
    Healthy,
    AuthorityCorrupt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDiagnosticIssue {
    pub code: String,
    pub detail: String,
}

pub(super) fn diagnose(repo: &RepoManager, repo_name: &str) -> Result<ProjectionDiagnostic> {
    match projection_plan::build(repo, repo_name) {
        Ok(_) => Ok(ProjectionDiagnostic {
            repo_name: repo_name.to_string(),
            status: ProjectionDiagnosticStatus::Healthy,
            issue: None,
            rebuild_supported: true,
            repair_hint: "projection authority is healthy; rebuild is available if workspace files need regeneration",
        }),
        Err(err) if materialize::is_broken_structure_projection_error(&err) => {
            let detail = err.to_string();
            Ok(ProjectionDiagnostic {
                repo_name: repo_name.to_string(),
                status: ProjectionDiagnosticStatus::AuthorityCorrupt,
                issue: Some(ProjectionDiagnosticIssue {
                    code: classify_authority_corruption(&detail).to_string(),
                    detail,
                }),
                rebuild_supported: false,
                repair_hint: "Structure Facts authority is corrupt; projection rebuild is unsupported, inspect ledger/backups before repair",
            })
        }
        Err(err) => Err(err),
    }
}

fn classify_authority_corruption(detail: &str) -> &'static str {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("missing parent") {
        return "missing_parent";
    }
    if lower.contains("missing node") {
        return "missing_node";
    }
    if lower.contains("cycle") {
        return "cycle";
    }
    if lower.contains("lost doc identity") || lower.contains("node/doc mismatch") {
        return "doc_identity";
    }
    if lower.contains("parent is not a directory") {
        return "invalid_parent_kind";
    }
    if lower.contains("duplicate create") {
        return "duplicate_node";
    }
    if lower.contains("path collision") {
        return "path_collision";
    }
    "structure_authority"
}
