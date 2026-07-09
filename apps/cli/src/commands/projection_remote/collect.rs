//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-scope
//!   - 06_backup#projection-backup-contract
//!   - 06_backup#projection-backup-upload-state-machine-contract

use anyhow::{Context, Result};
use deve_core::remote_projection::RemoteProjectionFile;
use deve_core::utils::path::path_to_forward_slash;
use deve_core::watcher_ignore::IgnoreRules;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn collect_markdown_projection_files(
    workspace: &Path,
) -> Result<Vec<MarkdownProjectionFileRef>> {
    let rules = IgnoreRules::load(workspace);
    let mut files = Vec::new();
    collect_markdown_projection_files_inner(workspace, workspace, &rules, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownProjectionFileRef {
    path: String,
    fs_path: PathBuf,
}

impl MarkdownProjectionFileRef {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn fs_path(&self) -> &Path {
        &self.fs_path
    }
}

fn collect_markdown_projection_files_inner(
    workspace: &Path,
    current: &Path,
    rules: &IgnoreRules,
    files: &mut Vec<MarkdownProjectionFileRef>,
) -> Result<()> {
    let mut entries = fs::read_dir(current)
        .with_context(|| format!("failed to read projection workspace {}", current.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| {
            format!(
                "failed to enumerate projection workspace {}",
                current.display()
            )
        })?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect projection path {}", path.display()))?;
        let name = entry.file_name();
        if is_reserved_projection_segment(&name.to_string_lossy()) {
            continue;
        }
        let rel = path
            .strip_prefix(workspace)
            .with_context(|| format!("failed to relativize projection path {}", path.display()))?;
        let rel = path_to_forward_slash(rel);
        if rules.is_ignored(&rel) || is_reserved_projection_path(&rel) {
            continue;
        }
        if file_type.is_dir() {
            collect_markdown_projection_files_inner(workspace, &path, rules, files)?;
            continue;
        }
        if !file_type.is_file() || !is_markdown_path(&rel) {
            continue;
        }
        RemoteProjectionFile::new(&rel, Vec::new())?;
        files.push(MarkdownProjectionFileRef {
            path: rel,
            fs_path: path,
        });
    }

    Ok(())
}

pub(super) fn is_markdown_path(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".markdown")
}

pub(super) fn is_reserved_projection_path(path: &str) -> bool {
    path.split('/').any(is_reserved_projection_segment)
}

fn is_reserved_projection_segment(segment: &str) -> bool {
    matches!(
        segment,
        ".git" | ".notegit" | "ledger" | "snapshot" | "snapshots" | "staging"
    )
}
