//! plan_ref:
//!   - 06_backup#backup-branch-binding-contract
//!   - 06_backup#backup-remote-layout-contract
//!
//! Backup branch discovery.
//!
//! This module derives readonly branch candidates from remote object paths. It
//! does not bind branches, call providers, verify manifests, upload/download
//! packs, write ledger state, or touch Projection Workspaces.

use super::layout::BackupRemoteObject;
use super::locator::{BackupLocator, normalize_remote_path, safe_writer_identity};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchDiscoveryInput {
    pub repo_locator: BackupLocator,
    pub objects: Vec<BackupRemoteObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchDiscoveryReport {
    pub repo_manifest_path: String,
    pub observed_object_count: usize,
    pub branches: Vec<DiscoveredBackupBranch>,
    pub diagnostics: Vec<BackupBranchDiscoveryDiagnostic>,
}

impl BackupBranchDiscoveryReport {
    pub fn is_healthy(&self) -> bool {
        self.diagnostics
            .iter()
            .all(BackupBranchDiscoveryDiagnostic::is_non_authoritative)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredBackupBranch {
    pub writer_identity: String,
    pub branch_path: String,
    pub branch_manifest_path: String,
    pub pack_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBranchDiscoveryDiagnostic {
    pub kind: BackupBranchDiscoveryDiagnosticKind,
    pub path: Option<String>,
    pub detail: Option<String>,
}

impl BackupBranchDiscoveryDiagnostic {
    fn new(kind: BackupBranchDiscoveryDiagnosticKind, path: Option<String>) -> Self {
        Self {
            kind,
            path,
            detail: None,
        }
    }

    fn with_detail(
        kind: BackupBranchDiscoveryDiagnosticKind,
        path: Option<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            path,
            detail: Some(detail.into()),
        }
    }

    fn is_non_authoritative(&self) -> bool {
        matches!(
            self.kind,
            BackupBranchDiscoveryDiagnosticKind::ProviderMetadataObserved
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupBranchDiscoveryDiagnosticKind {
    MissingRepoManifest,
    UnsafeRemoteObjectPath,
    UnsafeWriterIdentity,
    DuplicateObjectPath,
    DuplicateBranchManifest,
    OutsideRepoRoot,
    ProviderMetadataObserved,
}

pub fn discover_backup_branches(input: BackupBranchDiscoveryInput) -> BackupBranchDiscoveryReport {
    let repo_root_path = match normalize_remote_path(&input.repo_locator.repo_root_path) {
        Ok(path) => path,
        Err(error) => {
            return BackupBranchDiscoveryReport {
                repo_manifest_path: format!(
                    "{}/repo.manifest.enc",
                    input.repo_locator.repo_root_path
                ),
                observed_object_count: input.objects.len(),
                branches: Vec::new(),
                diagnostics: vec![BackupBranchDiscoveryDiagnostic::with_detail(
                    BackupBranchDiscoveryDiagnosticKind::UnsafeRemoteObjectPath,
                    Some(input.repo_locator.repo_root_path),
                    error.to_string(),
                )],
            };
        }
    };

    let repo_manifest_path = format!("{repo_root_path}/repo.manifest.enc");
    let branch_prefix = format!("{repo_root_path}/branches/");
    let mut diagnostics = Vec::new();
    let mut observed_paths = HashSet::new();
    let mut branch_writers = HashSet::new();
    let mut branches = Vec::new();

    for object in &input.objects {
        let path = match normalize_object_path(object, &mut diagnostics) {
            Some(path) => path,
            None => continue,
        };

        if !observed_paths.insert(path.clone()) {
            diagnostics.push(BackupBranchDiscoveryDiagnostic::new(
                BackupBranchDiscoveryDiagnosticKind::DuplicateObjectPath,
                Some(path.clone()),
            ));
        }
        if object.metadata.is_some() {
            diagnostics.push(BackupBranchDiscoveryDiagnostic::new(
                BackupBranchDiscoveryDiagnosticKind::ProviderMetadataObserved,
                Some(path.clone()),
            ));
        }

        if path == repo_manifest_path {
            continue;
        }
        if !path.starts_with(&format!("{repo_root_path}/")) {
            diagnostics.push(BackupBranchDiscoveryDiagnostic::new(
                BackupBranchDiscoveryDiagnosticKind::OutsideRepoRoot,
                Some(path),
            ));
            continue;
        }

        let Some(writer_identity) =
            branch_manifest_writer_identity(&path, &branch_prefix, &mut diagnostics)
        else {
            continue;
        };

        if !branch_writers.insert(writer_identity.clone()) {
            diagnostics.push(BackupBranchDiscoveryDiagnostic::new(
                BackupBranchDiscoveryDiagnosticKind::DuplicateBranchManifest,
                Some(path),
            ));
            continue;
        }

        let branch_path = format!("{branch_prefix}{writer_identity}");
        branches.push(DiscoveredBackupBranch {
            writer_identity,
            branch_manifest_path: path,
            pack_prefix: format!("{branch_path}/packs"),
            branch_path,
        });
    }

    if !observed_paths.contains(&repo_manifest_path) {
        diagnostics.push(BackupBranchDiscoveryDiagnostic::new(
            BackupBranchDiscoveryDiagnosticKind::MissingRepoManifest,
            Some(repo_manifest_path.clone()),
        ));
    }

    branches.sort_by(|left, right| left.writer_identity.cmp(&right.writer_identity));

    BackupBranchDiscoveryReport {
        repo_manifest_path,
        observed_object_count: input.objects.len(),
        branches,
        diagnostics,
    }
}

fn normalize_object_path(
    object: &BackupRemoteObject,
    diagnostics: &mut Vec<BackupBranchDiscoveryDiagnostic>,
) -> Option<String> {
    match normalize_remote_path(&object.path) {
        Ok(path) => Some(path),
        Err(error) => {
            diagnostics.push(BackupBranchDiscoveryDiagnostic::with_detail(
                BackupBranchDiscoveryDiagnosticKind::UnsafeRemoteObjectPath,
                Some(object.path.clone()),
                error.to_string(),
            ));
            None
        }
    }
}

fn branch_manifest_writer_identity(
    path: &str,
    branch_prefix: &str,
    diagnostics: &mut Vec<BackupBranchDiscoveryDiagnostic>,
) -> Option<String> {
    let rest = path.strip_prefix(branch_prefix)?;
    let writer_identity = rest.strip_suffix("/branch.manifest.enc")?;
    if writer_identity.contains('/') {
        diagnostics.push(BackupBranchDiscoveryDiagnostic::new(
            BackupBranchDiscoveryDiagnosticKind::UnsafeWriterIdentity,
            Some(path.to_string()),
        ));
        return None;
    }
    match safe_writer_identity(writer_identity) {
        Ok(writer_identity) => Some(writer_identity),
        Err(error) => {
            diagnostics.push(BackupBranchDiscoveryDiagnostic::with_detail(
                BackupBranchDiscoveryDiagnosticKind::UnsafeWriterIdentity,
                Some(path.to_string()),
                error.to_string(),
            ));
            None
        }
    }
}
