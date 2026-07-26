//! Bounded, fail-closed acceptance receipt artifact reader.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::file_identity::FileIdentity;
use crate::acceptance_matrix::receipt_limits::{
    MAX_RECEIPT_FILES, add_total_bytes, read_json_bounded,
};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_DIRECTORY_DEPTH: usize = 32;

#[derive(Default)]
pub(in crate::acceptance_matrix) struct ReceiptArtifactBudget {
    files: usize,
    bytes: u64,
}

impl ReceiptArtifactBudget {
    fn record(&mut self, bytes: u64) -> Result<()> {
        if self.files >= MAX_RECEIPT_FILES {
            bail!("acceptance receipts: JSON file limit exceeded");
        }
        add_total_bytes(
            "acceptance receipts: aggregate JSON size",
            &mut self.bytes,
            bytes,
        )?;
        self.files += 1;
        Ok(())
    }
}

pub(in crate::acceptance_matrix) struct ReceiptArtifactRoot {
    root: PathBuf,
    canonical_root: PathBuf,
    root_identity: FileIdentity,
}

impl ReceiptArtifactRoot {
    pub(in crate::acceptance_matrix) fn open(root: &Path) -> Result<Self> {
        if !root.is_dir() {
            bail!(
                "acceptance receipts: input directory is missing: {}",
                root.display()
            );
        }
        let metadata = fs::symlink_metadata(root)?;
        if is_reparse_or_symlink(&metadata) {
            bail!("acceptance receipts: root may not be a symlink or reparse point");
        }
        let canonical_root = fs::canonicalize(root)?;
        let reader = Self {
            root: root.to_path_buf(),
            root_identity: FileIdentity::read(&canonical_root)?,
            canonical_root,
        };
        reader.validate_root_identity()?;
        Ok(reader)
    }

    pub(in crate::acceptance_matrix) fn json_files(&self) -> Result<Vec<(String, PathBuf)>> {
        self.validate_root_identity()?;
        let receipts_root = self.root.join("receipts");
        let receipts_metadata = fs::symlink_metadata(&receipts_root)
            .context("acceptance receipts: inspect receipts directory")?;
        if !receipts_metadata.is_dir() || is_reparse_or_symlink(&receipts_metadata) {
            bail!("acceptance receipts: receipts directory must be a real directory");
        }
        self.validate_entry(&receipts_root)?;
        let canonical_receipts = fs::canonicalize(&receipts_root)?;
        let receipts_identity = FileIdentity::read(&canonical_receipts)?;

        let mut stack = vec![(receipts_root.clone(), 0usize)];
        let mut files = Vec::new();
        let mut visited = 0usize;
        while let Some((directory, depth)) = stack.pop() {
            if depth > MAX_DIRECTORY_DEPTH {
                bail!("acceptance receipts: directory depth exceeds {MAX_DIRECTORY_DEPTH}");
            }
            self.validate_entry(&directory)?;
            for entry in fs::read_dir(&directory)? {
                visited += 1;
                if visited > MAX_RECEIPT_FILES * 4 {
                    bail!("acceptance receipts: directory entry limit exceeded");
                }
                let entry = entry?;
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path)?;
                let file_type = metadata.file_type();
                if is_reparse_or_symlink(&metadata) {
                    bail!(
                        "acceptance receipts: symlink/reparse entry is forbidden: {}",
                        path.display()
                    );
                }
                self.validate_entry(&path)?;
                if file_type.is_dir() {
                    stack.push((path, depth + 1));
                } else if file_type.is_file()
                    && path.extension().and_then(|value| value.to_str()) == Some("json")
                {
                    if files.len() >= MAX_RECEIPT_FILES {
                        bail!("acceptance receipts: JSON file limit exceeded");
                    }
                    files.push((canonical_relative(&self.root, &path)?, path));
                }
            }
        }
        self.validate_root_identity()?;
        self.validate_entry(&receipts_root)?;
        if fs::canonicalize(&receipts_root)? != canonical_receipts
            || FileIdentity::read(&canonical_receipts)? != receipts_identity
        {
            bail!("acceptance receipts: receipts directory identity changed");
        }
        files.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(files)
    }

    pub(in crate::acceptance_matrix) fn read_json(
        &self,
        path: &Path,
        budget: &mut ReceiptArtifactBudget,
    ) -> Result<Vec<u8>> {
        self.validate_root_identity()?;
        self.validate_entry(path)?;
        let content = read_json_bounded(path, "acceptance receipt JSON")?;
        self.validate_entry(path)?;
        self.validate_root_identity()?;
        budget.record(content.len() as u64)?;
        Ok(content)
    }

    fn validate_root_identity(&self) -> Result<()> {
        let metadata = fs::symlink_metadata(&self.root)?;
        if is_reparse_or_symlink(&metadata) {
            bail!("acceptance receipts: input root identity changed");
        }
        let current = fs::canonicalize(&self.root)?;
        if current != self.canonical_root {
            bail!("acceptance receipts: input root identity changed");
        }
        if FileIdentity::read(&current)? != self.root_identity {
            bail!("acceptance receipts: input root identity changed");
        }
        Ok(())
    }

    fn validate_entry(&self, path: &Path) -> Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        if is_reparse_or_symlink(&metadata) {
            bail!(
                "acceptance receipts: symlink/reparse entry is forbidden: {}",
                path.display()
            );
        }
        let canonical = fs::canonicalize(path)?;
        if !canonical.starts_with(&self.canonical_root) {
            bail!(
                "acceptance receipts: entry escaped input root: {}",
                path.display()
            );
        }
        Ok(())
    }
}

fn canonical_relative(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .context("acceptance receipt escaped input root")?;
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("acceptance receipt locator is not a canonical relative path");
    }
    let value = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    if !value.starts_with("receipts/") {
        bail!("acceptance receipt locator must live below receipts/");
    }
    Ok(value)
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
    use super::{ReceiptArtifactBudget, ReceiptArtifactRoot};
    use crate::acceptance_matrix::receipt_limits::MAX_RECEIPT_BYTES;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("deve-acceptance-artifacts-{unique}"))
    }

    #[test]
    fn artifact_reader_lists_only_canonical_receipt_locators() {
        let root = temp_root();
        fs::create_dir_all(root.join("receipts/nested")).unwrap();
        fs::create_dir_all(root.join("state/playwright/node_modules")).unwrap();
        fs::write(root.join("receipts/nested/one.json"), b"{}").unwrap();
        fs::write(root.join("receipts/ignored.txt"), b"ignored").unwrap();
        fs::write(root.join("state/playwright/package.json"), b"{}").unwrap();
        fs::write(
            root.join("state/playwright/node_modules/metadata.json"),
            b"{}",
        )
        .unwrap();
        let reader = ReceiptArtifactRoot::open(&root).unwrap();

        let files = reader.json_files().unwrap();

        assert_eq!(
            files
                .iter()
                .map(|(relative, _)| relative.as_str())
                .collect::<Vec<_>>(),
            ["receipts/nested/one.json"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn artifact_reader_requires_a_receipts_subtree() {
        let root = temp_root();
        fs::create_dir_all(root.join("state")).unwrap();
        fs::write(root.join("state/package.json"), b"{}").unwrap();
        let reader = ReceiptArtifactRoot::open(&root).unwrap();

        let error = reader.json_files().unwrap_err();

        assert!(error.to_string().contains("receipts directory"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn artifact_reader_rejects_an_oversized_receipt_before_allocation() {
        let root = temp_root();
        let receipt = root.join("receipts/large.json");
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        let file = fs::File::create(&receipt).unwrap();
        file.set_len(MAX_RECEIPT_BYTES + 1).unwrap();
        let reader = ReceiptArtifactRoot::open(&root).unwrap();

        let error = reader
            .read_json(&receipt, &mut ReceiptArtifactBudget::default())
            .unwrap_err();

        assert!(error.to_string().contains("exceeds"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn artifact_reader_rejects_same_path_root_replacement() {
        let root = temp_root();
        let original = root.with_file_name(format!(
            "{}-original",
            root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(root.join("receipts")).unwrap();
        let reader = ReceiptArtifactRoot::open(&root).unwrap();
        fs::rename(&root, &original).unwrap();
        fs::create_dir_all(root.join("receipts")).unwrap();

        let error = reader.json_files().unwrap_err();

        assert!(error.to_string().contains("identity changed"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(original);
    }

    #[test]
    fn artifact_reader_rejects_excessive_directory_depth() {
        let root = temp_root();
        let mut directory = root.join("receipts");
        for _ in 0..33 {
            directory = directory.join("d");
        }
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("receipt.json"), b"{}").unwrap();
        let reader = ReceiptArtifactRoot::open(&root).unwrap();

        let error = reader.json_files().unwrap_err();

        assert!(error.to_string().contains("directory depth exceeds 32"));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn artifact_reader_rejects_symlink_entries() {
        use std::os::unix::fs::symlink;

        let root = temp_root();
        let outside = root.with_extension("outside");
        fs::create_dir_all(root.join("receipts")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("receipts/link")).unwrap();
        let reader = ReceiptArtifactRoot::open(&root).unwrap();

        let error = reader.json_files().unwrap_err();

        assert!(
            error
                .to_string()
                .contains("symlink/reparse entry is forbidden")
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(windows)]
    #[test]
    fn artifact_reader_rejects_reparse_entries_when_symlinks_are_available() {
        use std::os::windows::fs::symlink_dir;

        let root = temp_root();
        let outside = root.with_extension("outside");
        fs::create_dir_all(root.join("receipts")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        if symlink_dir(&outside, root.join("receipts/link")).is_err() {
            let _ = fs::remove_dir_all(root);
            let _ = fs::remove_dir_all(outside);
            return;
        }
        let reader = ReceiptArtifactRoot::open(&root).unwrap();

        let error = reader.json_files().unwrap_err();

        assert!(
            error
                .to_string()
                .contains("symlink/reparse entry is forbidden")
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }
}
