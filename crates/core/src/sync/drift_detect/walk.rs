//! plan_ref:
//!   - 04_storage.md §7. Projection and Persistence Contract

use super::EntryKind;
use crate::ledger::RepoManager;
use crate::source_control::pending_fs;
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
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
    let mut entries = BTreeMap::new();
    if root.exists() {
        walk_dir(root.as_path(), "", &mut entries)?;
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
        if name == ".notegit" {
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
        if file_type.is_file() {
            let content = std::fs::read_to_string(entry.path())?;
            entries.insert(
                to_forward_slash(&path),
                WorkspaceEntry {
                    kind: EntryKind::File,
                    content_hash: Some(pending_fs::content_hash(&content)),
                },
            );
            continue;
        }
        return Err(anyhow!(
            "unsupported workspace entry at {}",
            entry.path().display()
        ));
    }

    Ok(())
}
