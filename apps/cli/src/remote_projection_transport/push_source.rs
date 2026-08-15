//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!   - 06_backup#projection-backup-scope
//!   - 06_backup#projection-backup-contract
//!   - 06_backup#projection-backup-upload-state-machine-contract

use anyhow::{Context, Result};
use deve_core::remote_projection::{RemoteProjectionFile, RemoteProjectionProviderError};
use deve_core::utils::fs::{HostPathIdentity, HostPathKind, open_regular_file_read};
use deve_core::utils::path::path_to_forward_slash;
use deve_core::watcher_ignore::IgnoreRules;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::path_set::{
    MAX_SOURCE_FILE_BYTES, MAX_SOURCE_FILES, MAX_SOURCE_TOTAL_BYTES, NormalizedRemotePath,
    RemotePathBudget,
};

pub(crate) fn collect_markdown_projection_files(
    workspace: &Path,
) -> Result<Vec<MarkdownProjectionFileRef>> {
    let rules = IgnoreRules::load(workspace);
    let mut files = Vec::new();
    let mut budget = PushSourceBudget::default();
    collect_markdown_projection_files_inner(workspace, workspace, &rules, &mut budget, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

pub(crate) struct WorkspaceProjectionPushSource {
    files: Vec<MarkdownProjectionFileRef>,
}

impl WorkspaceProjectionPushSource {
    pub(crate) fn collect(workspace: &Path) -> Result<Self> {
        Ok(Self {
            files: collect_markdown_projection_files(workspace)?,
        })
    }
}

impl super::ProjectionPushSource for WorkspaceProjectionPushSource {
    fn file_count(&self) -> usize {
        self.files.len()
    }

    fn visit(
        &self,
        visitor: &mut super::ProjectionPushVisitor<'_>,
    ) -> Result<(), RemoteProjectionProviderError> {
        let mut total_bytes = 0usize;
        for file in &self.files {
            total_bytes = observe_payload_budget(total_bytes, file.expected_len)?;
            let content = read_exact_projection_file(file)?;
            let actual_sha256: [u8; 32] = Sha256::digest(&content).into();
            if actual_sha256 != file.expected_sha256 {
                return Err(push_budget_error(
                    "projection push source content changed after enumeration",
                ));
            }
            visitor(file.path(), content)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownProjectionFileRef {
    path: String,
    fs_path: PathBuf,
    expected_len: usize,
    expected_identity: HostPathIdentity,
    expected_sha256: [u8; 32],
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
    budget: &mut PushSourceBudget,
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
            collect_markdown_projection_files_inner(workspace, &path, rules, budget, files)?;
            continue;
        }
        if !file_type.is_file() || !is_markdown_path(&rel) {
            continue;
        }
        let normalized = NormalizedRemotePath::new(&rel)?;
        RemoteProjectionFile::new(normalized.as_str(), Vec::new())?;
        let expected_identity = HostPathIdentity::capture(&path, HostPathKind::RegularFile)
            .with_context(|| format!("failed to capture projection file {}", path.display()))?;
        let mut file = open_regular_file_read(&path, "projection push source")
            .with_context(|| format!("failed to open projection file {}", path.display()))?;
        if !expected_identity.matches_open_file(&file)? {
            anyhow::bail!("projection push source changed during enumeration");
        }
        let metadata = file
            .metadata()
            .with_context(|| format!("failed to inspect projection file {}", path.display()))?;
        let expected_len = usize::try_from(metadata.len())
            .map_err(|_| anyhow::anyhow!("projection push source length exceeds host usize"))?;
        budget.observe(normalized.as_str(), expected_len)?;
        let content = read_bounded(&mut file, expected_len)
            .with_context(|| format!("failed to read projection file {}", path.display()))?;
        let expected_sha256: [u8; 32] = Sha256::digest(&content).into();
        files.push(MarkdownProjectionFileRef {
            path: normalized.as_str().to_string(),
            fs_path: path,
            expected_len,
            expected_identity,
            expected_sha256,
        });
    }

    Ok(())
}

fn read_exact_projection_file(
    file: &MarkdownProjectionFileRef,
) -> Result<Vec<u8>, RemoteProjectionProviderError> {
    let mut opened = open_regular_file_read(file.fs_path(), "projection push source")
        .map_err(|_| push_budget_error("projection push source changed after enumeration"))?;
    let identity_matches = file
        .expected_identity
        .matches_open_file(&opened)
        .map_err(|_| push_budget_error("projection push source identity is unavailable"))?;
    if !identity_matches {
        return Err(push_budget_error(
            "projection push source changed after enumeration",
        ));
    }
    let metadata = opened
        .metadata()
        .map_err(|_| push_budget_error("projection push source metadata is unavailable"))?;
    if metadata.len() != file.expected_len as u64 {
        return Err(push_budget_error(
            "projection push source changed after enumeration",
        ));
    }
    read_bounded(&mut opened, file.expected_len)
        .map_err(|_| push_budget_error("projection push source read failed or exceeded its budget"))
}

fn read_bounded(reader: &mut impl Read, expected_len: usize) -> std::io::Result<Vec<u8>> {
    let limit = u64::try_from(expected_len)
        .ok()
        .and_then(|length| length.checked_add(1))
        .ok_or_else(|| std::io::Error::other("projection push source length overflow"))?;
    let mut content = Vec::with_capacity(expected_len.min(MAX_SOURCE_FILE_BYTES));
    reader.take(limit).read_to_end(&mut content)?;
    if content.len() != expected_len {
        return Err(std::io::Error::other(
            "projection push source length changed while reading",
        ));
    }
    Ok(content)
}

struct PushSourceBudget {
    files: usize,
    total_bytes: usize,
    paths: RemotePathBudget,
    casefold_paths: BTreeSet<String>,
}

impl Default for PushSourceBudget {
    fn default() -> Self {
        Self {
            files: 0,
            total_bytes: 0,
            paths: RemotePathBudget::new("projection push source"),
            casefold_paths: BTreeSet::new(),
        }
    }
}

impl PushSourceBudget {
    fn observe(&mut self, path: &str, bytes: usize) -> Result<()> {
        if self.files == MAX_SOURCE_FILES {
            return Err(push_budget_error(format!(
                "projection push source exceeds file budget of {MAX_SOURCE_FILES}"
            ))
            .into());
        }
        if !self.casefold_paths.insert(path.to_lowercase()) {
            return Err(RemoteProjectionProviderError::DuplicateProjectionPath.into());
        }
        self.paths.observe(path)?;
        self.total_bytes = observe_payload_budget(self.total_bytes, bytes)?;
        self.files += 1;
        Ok(())
    }
}

fn observe_payload_budget(
    current_total: usize,
    file_bytes: usize,
) -> Result<usize, RemoteProjectionProviderError> {
    if file_bytes > MAX_SOURCE_FILE_BYTES {
        return Err(push_budget_error(format!(
            "projection push source exceeds per-file byte budget of {MAX_SOURCE_FILE_BYTES}"
        )));
    }
    let next_total = current_total
        .checked_add(file_bytes)
        .ok_or_else(|| push_budget_error("projection push source total bytes overflow"))?;
    if next_total > MAX_SOURCE_TOTAL_BYTES {
        return Err(push_budget_error(format!(
            "projection push source exceeds total byte budget of {MAX_SOURCE_TOTAL_BYTES}"
        )));
    }
    Ok(next_total)
}

fn push_budget_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}

pub(super) fn is_markdown_path(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".markdown")
}

pub(super) fn is_reserved_projection_path(path: &str) -> bool {
    path.split('/').any(is_reserved_projection_segment)
}

fn is_reserved_projection_segment(segment: &str) -> bool {
    [
        ".git",
        ".notegit",
        "ledger",
        "snapshot",
        "snapshots",
        "staging",
    ]
    .iter()
    .any(|reserved| segment.eq_ignore_ascii_case(reserved))
}

#[cfg(test)]
mod tests {
    use super::{MAX_SOURCE_FILE_BYTES, WorkspaceProjectionPushSource};
    use crate::remote_projection_transport::ProjectionPushSource;

    fn sparse_file(path: &std::path::Path, bytes: u64) {
        let file = std::fs::File::create(path).expect("create sparse file");
        file.set_len(bytes).expect("size sparse file");
    }

    #[test]
    fn projection_push_source_rejects_single_and_total_payload_over_budget() {
        let single = tempfile::tempdir().expect("single tempdir");
        sparse_file(
            &single.path().join("oversized.md"),
            MAX_SOURCE_FILE_BYTES as u64 + 1,
        );
        assert!(WorkspaceProjectionPushSource::collect(single.path()).is_err());

        let total = tempfile::tempdir().expect("total tempdir");
        for index in 0..17 {
            sparse_file(
                &total.path().join(format!("{index}.md")),
                MAX_SOURCE_FILE_BYTES as u64,
            );
        }
        assert!(WorkspaceProjectionPushSource::collect(total.path()).is_err());
    }

    #[test]
    fn projection_push_source_rejects_file_growth_after_enumeration() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("note.md");
        std::fs::write(&path, "small").expect("write");
        let source = WorkspaceProjectionPushSource::collect(dir.path()).expect("collect");
        sparse_file(&path, MAX_SOURCE_FILE_BYTES as u64 + 1);
        let error = source
            .visit(&mut |_, _| Ok(()))
            .expect_err("growth must fail");
        assert!(error.to_string().contains("changed after enumeration"));
    }

    #[test]
    fn projection_push_source_rejects_same_length_file_replacement() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("note.md");
        std::fs::write(&path, "first").expect("write");
        let source = WorkspaceProjectionPushSource::collect(dir.path()).expect("collect");
        std::fs::remove_file(&path).expect("remove original");
        std::fs::write(&path, "other").expect("replace same length");
        let error = source
            .visit(&mut |_, _| Ok(()))
            .expect_err("replacement must fail");
        assert!(error.to_string().contains("changed after enumeration"));
    }

    #[test]
    fn projection_push_source_rejects_same_inode_same_length_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("note.md");
        std::fs::write(&path, "first").expect("write");
        let source = WorkspaceProjectionPushSource::collect(dir.path()).expect("collect");
        std::fs::write(&path, "other").expect("overwrite same file and length");

        let error = source
            .visit(&mut |_, _| Ok(()))
            .expect_err("same-inode content replacement must fail");

        assert!(error.to_string().contains("changed after enumeration"));
    }
}
