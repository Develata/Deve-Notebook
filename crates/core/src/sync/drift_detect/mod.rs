//! plan_ref:
//!   - 03_storage/projection#projection-contract

mod enumerate;
mod explain;
mod walk;

use crate::ledger::RepoManager;
use crate::source_control::{pending_fs, staging};
use anyhow::Result;
use explain::{DiffEvidence, is_explained};
use std::path::Path;

#[derive(Debug, Default)]
pub struct DriftReport {
    pub unexplained: Vec<DriftEntry>,
    pub explained_count: usize,
}

impl DriftReport {
    pub fn is_fault(&self) -> bool {
        !self.unexplained.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftEntry {
    pub path: String,
    pub kind: DriftKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftKind {
    MissingOnDisk,
    UnexpectedOnDisk,
    ContentMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntryKind {
    File,
    Dir,
}

pub fn detect_repo_drift(repo: &RepoManager, repo_name: &str) -> Result<DriftReport> {
    let projection = enumerate::enumerate_projection(repo, repo_name)?;
    let workspace = walk::enumerate_workspace(repo, repo_name)?;
    detect_repo_drift_from_entries(repo, repo_name, projection, workspace)
}

pub fn detect_repo_drift_at_workspace_root(
    repo: &RepoManager,
    repo_name: &str,
    workspace_root: &Path,
) -> Result<DriftReport> {
    let projection = enumerate::enumerate_projection(repo, repo_name)?;
    let workspace = walk::enumerate_workspace_root(workspace_root)?;
    detect_repo_drift_from_entries(repo, repo_name, projection, workspace)
}

fn detect_repo_drift_from_entries(
    repo: &RepoManager,
    repo_name: &str,
    projection: std::collections::BTreeMap<String, enumerate::ProjectedEntry>,
    workspace: std::collections::BTreeMap<String, walk::WorkspaceEntry>,
) -> Result<DriftReport> {
    let pending = repo.run_on_local_repo(repo_name, pending_fs::list_all)?;
    let staged = repo.run_on_local_repo(repo_name, staging::list_staged_entries)?;
    let mut report = DriftReport::default();

    for (path, expected) in &projection {
        match workspace.get(path) {
            Some(actual) if actual.kind == expected.kind => {
                if actual.kind == EntryKind::File && actual.content_hash != expected.content_hash {
                    record_diff(
                        &mut report,
                        path,
                        DriftKind::ContentMismatch,
                        actual.content_hash.as_deref(),
                        &pending,
                        &staged,
                    );
                }
            }
            _ => record_diff(
                &mut report,
                path,
                DriftKind::MissingOnDisk,
                None,
                &pending,
                &staged,
            ),
        }
    }

    for (path, actual) in &workspace {
        if projection
            .get(path)
            .is_some_and(|expected| expected.kind == actual.kind)
        {
            continue;
        }
        record_diff(
            &mut report,
            path,
            DriftKind::UnexpectedOnDisk,
            actual.content_hash.as_deref(),
            &pending,
            &staged,
        );
    }

    Ok(report)
}

fn record_diff(
    report: &mut DriftReport,
    path: &str,
    kind: DriftKind,
    hash: Option<&str>,
    pending: &[pending_fs::PendingFsEntry],
    staged: &[(String, staging::StagedEntry)],
) {
    let evidence = DiffEvidence {
        path,
        kind,
        workspace_hash: hash,
    };
    if is_explained(evidence, pending, staged) {
        report.explained_count += 1;
    } else {
        report.unexplained.push(DriftEntry {
            path: path.to_string(),
            kind,
        });
    }
}
