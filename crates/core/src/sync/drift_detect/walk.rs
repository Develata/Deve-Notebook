//! plan_ref:
//!   - 03_storage#projection-contract

use super::EntryKind;
use crate::ledger::RepoManager;
use crate::source_control::pending_fs;
use crate::utils::path::to_forward_slash;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WorkspaceEntry {
    pub kind: EntryKind,
    pub content_hash: Option<String>,
}

pub fn enumerate_workspace(
    repo: &RepoManager,
    repo_name: &str,
) -> Result<BTreeMap<String, WorkspaceEntry>> {
    let root = repo.local_repo_workspace_root(repo_name)?;
    enumerate_workspace_root(&root)
}

pub(super) fn enumerate_workspace_root(root: &Path) -> Result<BTreeMap<String, WorkspaceEntry>> {
    let mut entries = BTreeMap::new();
    if root.exists() {
        walk_dir(root, "", &mut entries)?;
    }
    Ok(entries)
}

fn walk_dir(
    root: &Path,
    relative: &str,
    entries: &mut BTreeMap<String, WorkspaceEntry>,
) -> Result<()> {
    let current = if relative.is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    };

    for entry in std::fs::read_dir(&current)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if crate::utils::notegit::is_internal_repo_segment(&name) {
            continue;
        }
        if relative.is_empty() && name == ".gitignore" {
            continue;
        }
        let path = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            entries.insert(
                to_forward_slash(&path),
                WorkspaceEntry {
                    kind: EntryKind::Dir,
                    content_hash: None,
                },
            );
            walk_dir(root, &path, entries)?;
            continue;
        }
        entries.insert(
            to_forward_slash(&path),
            WorkspaceEntry {
                kind: EntryKind::File,
                content_hash: file_hash(entry.path().as_path())?,
            },
        );
    }

    Ok(())
}

fn file_hash(path: &Path) -> Result<Option<String>> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read workspace entry {}", path.display()))?;
    Ok(std::str::from_utf8(&bytes)
        .ok()
        .map(pending_fs::content_hash))
}
