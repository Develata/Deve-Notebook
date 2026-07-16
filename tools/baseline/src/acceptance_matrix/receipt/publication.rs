//! Reparse-aware, rollback-safe receipt publication.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use crate::acceptance_matrix::receipt_limits::{
    MAX_RECEIPT_FILES, add_total_bytes, validate_file_size,
};
use anyhow::{Context, Result, bail};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(in crate::acceptance_matrix) fn ensure_output_outside_worktree(
    root: &Path,
    output: &Path,
) -> Result<()> {
    let absolute = if output.is_absolute() {
        output.to_path_buf()
    } else {
        root.join(output)
    };
    if absolute.starts_with(root) {
        bail!("acceptance receipt output must be outside the Git worktree");
    }
    let canonical_root = fs::canonicalize(root)?;
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .context("acceptance receipt output has no existing ancestor")?;
    }
    for existing in ancestor.ancestors().filter(|path| path.exists()) {
        let metadata = fs::symlink_metadata(existing)?;
        if is_reparse_or_symlink(&metadata) {
            bail!(
                "acceptance receipt output ancestor is a symlink/reparse point: {}",
                existing.display()
            );
        }
    }
    let canonical_ancestor = fs::canonicalize(ancestor)?;
    if canonical_ancestor.starts_with(canonical_root) {
        bail!("acceptance receipt output resolves inside the Git worktree");
    }
    Ok(())
}

pub(super) fn write_batch_atomic(
    root: &Path,
    files: &[(PathBuf, Vec<u8>)],
    execution_id: &str,
) -> Result<()> {
    validate_batch(files)?;
    let mut pending = Vec::new();
    for (index, (path, content)) in files.iter().enumerate() {
        match stage_file(root, path, content, execution_id, index) {
            Ok(temp) => pending.push((temp, path.clone())),
            Err(error) => {
                cleanup_pending(&pending, 0);
                return Err(error);
            }
        }
    }

    for index in 0..pending.len() {
        let (temp, output) = &pending[index];
        if let Err(error) = ensure_output_outside_worktree(root, output) {
            cleanup_pending(&pending, index);
            return Err(error);
        }
        if output.exists() {
            cleanup_pending(&pending, index);
            bail!(
                "acceptance-receipt: output appeared before publication: {}",
                output.display()
            );
        }
        if let Err(error) = fs::rename(temp, output) {
            cleanup_pending(&pending, index);
            return Err(error).with_context(|| {
                format!("acceptance-receipt: failed to publish {}", output.display())
            });
        }
    }
    Ok(())
}

fn validate_batch(files: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    if files.len() > MAX_RECEIPT_FILES {
        bail!("acceptance-receipt: receipt file count exceeds {MAX_RECEIPT_FILES}");
    }
    let mut total = 0u64;
    for (_, content) in files {
        validate_file_size(
            "acceptance-receipt: serialized receipt",
            content.len() as u64,
        )?;
        add_total_bytes(
            "acceptance-receipt: serialized receipt group",
            &mut total,
            content.len() as u64,
        )?;
    }
    Ok(())
}

fn stage_file(
    root: &Path,
    path: &Path,
    content: &[u8],
    execution_id: &str,
    index: usize,
) -> Result<PathBuf> {
    ensure_output_outside_worktree(root, path)?;
    if path.exists() {
        bail!(
            "acceptance-receipt: output appeared during execution: {}",
            path.display()
        );
    }
    let parent = path
        .parent()
        .context("acceptance-receipt: output has no parent")?;
    fs::create_dir_all(parent)?;
    ensure_output_outside_worktree(root, path)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("receipt.json");
    let temp = path.with_file_name(format!(".{file_name}.{execution_id}.{index}.tmp"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .with_context(|| format!("acceptance-receipt: failed to create {}", temp.display()))?;
    if let Err(error) = file.write_all(content).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(error)
            .with_context(|| format!("acceptance-receipt: failed to write {}", temp.display()));
    }
    Ok(temp)
}

fn cleanup_pending(pending: &[(PathBuf, PathBuf)], published_count: usize) {
    for (index, (temp, output)) in pending.iter().enumerate() {
        let _ = fs::remove_file(temp);
        if index < published_count {
            let _ = fs::remove_file(output);
        }
    }
}

fn is_reparse_or_symlink(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_output_outside_worktree, validate_batch};
    use crate::acceptance_matrix::receipt_limits::MAX_RECEIPT_BYTES;

    #[test]
    fn receipt_output_inside_worktree_is_rejected() {
        let root = std::env::current_dir().unwrap();
        assert!(ensure_output_outside_worktree(&root, &root.join("receipt.json")).is_err());
    }

    #[test]
    fn publication_rejects_an_oversized_serialized_receipt() {
        let files = vec![(
            std::path::PathBuf::from("receipt.json"),
            vec![0; MAX_RECEIPT_BYTES as usize + 1],
        )];

        assert!(validate_batch(&files).is_err());
    }

    #[test]
    fn publication_rejects_a_group_over_the_aggregate_limit() {
        let files = (0..17)
            .map(|index| {
                (
                    std::path::PathBuf::from(format!("receipt-{index}.json")),
                    vec![0; MAX_RECEIPT_BYTES as usize],
                )
            })
            .collect::<Vec<_>>();

        assert!(validate_batch(&files).is_err());
    }
}
