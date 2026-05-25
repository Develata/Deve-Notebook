//! plan_ref:
//!   - 06_backup#backup-remote-layout-contract
//!
//! Backup remote layout diagnostics.
//!
//! This module compares provider object listings against the expected backup
//! layout. Provider metadata is retained only as transport diagnostics; it is
//! never treated as repo, branch, ledger, or pack authority.

use super::locator::{BranchBackupLocator, normalize_remote_path};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupTransportMetadata {
    pub etag: Option<String>,
    pub version: Option<String>,
    pub mtime_unix_ms: Option<i64>,
    pub object_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRemoteObject {
    pub path: String,
    pub metadata: Option<BackupTransportMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRemoteLayoutInput {
    pub branch: BranchBackupLocator,
    pub objects: Vec<BackupRemoteObject>,
    pub expected_pack_object_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRemoteLayoutReport {
    pub repo_manifest_path: String,
    pub branch_manifest_path: String,
    pub pack_prefix: String,
    pub observed_object_count: usize,
    pub diagnostics: Vec<BackupRemoteLayoutDiagnostic>,
}

impl BackupRemoteLayoutReport {
    pub fn is_healthy(&self) -> bool {
        self.diagnostics
            .iter()
            .all(BackupRemoteLayoutDiagnostic::is_non_authoritative)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRemoteLayoutDiagnostic {
    pub kind: BackupRemoteLayoutDiagnosticKind,
    pub path: Option<String>,
    pub detail: Option<String>,
}

impl BackupRemoteLayoutDiagnostic {
    fn new(kind: BackupRemoteLayoutDiagnosticKind, path: Option<String>) -> Self {
        Self {
            kind,
            path,
            detail: None,
        }
    }

    fn with_detail(
        kind: BackupRemoteLayoutDiagnosticKind,
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
            BackupRemoteLayoutDiagnosticKind::ProviderMetadataObserved
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupRemoteLayoutDiagnosticKind {
    MissingRepoManifest,
    MissingBranchManifest,
    MissingPack,
    UnexpectedPath,
    UnsafeRemoteObjectPath,
    DuplicateObjectPath,
    PackOutsideBranchPrefix,
    ProviderMetadataObserved,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupRemoteLayoutError {
    #[error("expected backup pack object path is outside the branch pack prefix")]
    ExpectedPackOutsideBranchPrefix,
    #[error("expected backup pack object path is duplicated")]
    DuplicateExpectedPackPath,
    #[error(transparent)]
    Locator(#[from] super::BackupLocatorError),
}

pub fn inspect_backup_remote_layout(
    input: BackupRemoteLayoutInput,
) -> Result<BackupRemoteLayoutReport, BackupRemoteLayoutError> {
    let repo_manifest_path = repo_manifest_path(&input.branch)?;
    let branch_manifest_path = input.branch.branch_manifest_path();
    let pack_prefix = input.branch.pack_prefix();
    let expected_pack_object_paths =
        normalize_expected_pack_paths(&pack_prefix, input.expected_pack_object_paths)?;

    let mut diagnostics = Vec::new();
    let mut observed_paths = HashSet::new();
    let mut authoritative_paths = HashSet::new();
    authoritative_paths.insert(repo_manifest_path.clone());
    authoritative_paths.insert(branch_manifest_path.clone());
    authoritative_paths.extend(expected_pack_object_paths.iter().cloned());

    for object in &input.objects {
        match normalize_remote_path(&object.path) {
            Ok(path) => {
                if !observed_paths.insert(path.clone()) {
                    diagnostics.push(BackupRemoteLayoutDiagnostic::new(
                        BackupRemoteLayoutDiagnosticKind::DuplicateObjectPath,
                        Some(path.clone()),
                    ));
                }
                if object.metadata.is_some() {
                    diagnostics.push(BackupRemoteLayoutDiagnostic::new(
                        BackupRemoteLayoutDiagnosticKind::ProviderMetadataObserved,
                        Some(path.clone()),
                    ));
                }
                if !authoritative_paths.contains(&path) {
                    diagnostics.push(unexpected_path_diagnostic(&pack_prefix, path));
                }
            }
            Err(error) => diagnostics.push(BackupRemoteLayoutDiagnostic::with_detail(
                BackupRemoteLayoutDiagnosticKind::UnsafeRemoteObjectPath,
                Some(object.path.clone()),
                error.to_string(),
            )),
        }
    }

    if !observed_paths.contains(&repo_manifest_path) {
        diagnostics.push(BackupRemoteLayoutDiagnostic::new(
            BackupRemoteLayoutDiagnosticKind::MissingRepoManifest,
            Some(repo_manifest_path.clone()),
        ));
    }
    if !observed_paths.contains(&branch_manifest_path) {
        diagnostics.push(BackupRemoteLayoutDiagnostic::new(
            BackupRemoteLayoutDiagnosticKind::MissingBranchManifest,
            Some(branch_manifest_path.clone()),
        ));
    }
    for pack_path in &expected_pack_object_paths {
        if !observed_paths.contains(pack_path) {
            diagnostics.push(BackupRemoteLayoutDiagnostic::new(
                BackupRemoteLayoutDiagnosticKind::MissingPack,
                Some(pack_path.clone()),
            ));
        }
    }

    Ok(BackupRemoteLayoutReport {
        repo_manifest_path,
        branch_manifest_path,
        pack_prefix,
        observed_object_count: input.objects.len(),
        diagnostics,
    })
}

fn repo_manifest_path(branch: &BranchBackupLocator) -> Result<String, BackupRemoteLayoutError> {
    Ok(format!(
        "{}/repo.manifest.enc",
        normalize_remote_path(&branch.root.repo_root_path)?
    ))
}

fn normalize_expected_pack_paths(
    pack_prefix: &str,
    expected_pack_object_paths: Vec<String>,
) -> Result<Vec<String>, BackupRemoteLayoutError> {
    let mut seen = HashSet::with_capacity(expected_pack_object_paths.len());
    expected_pack_object_paths
        .into_iter()
        .map(|path| {
            let path = normalize_remote_path(&path)?;
            if path == pack_prefix || !path.starts_with(&format!("{pack_prefix}/")) {
                return Err(BackupRemoteLayoutError::ExpectedPackOutsideBranchPrefix);
            }
            if !seen.insert(path.clone()) {
                return Err(BackupRemoteLayoutError::DuplicateExpectedPackPath);
            }
            Ok(path)
        })
        .collect()
}

fn unexpected_path_diagnostic(pack_prefix: &str, path: String) -> BackupRemoteLayoutDiagnostic {
    let kind = if path.starts_with(&format!("{pack_prefix}/")) {
        BackupRemoteLayoutDiagnosticKind::UnexpectedPath
    } else {
        BackupRemoteLayoutDiagnosticKind::PackOutsideBranchPrefix
    };
    BackupRemoteLayoutDiagnostic::new(kind, Some(path))
}
