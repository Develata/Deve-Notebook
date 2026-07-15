use super::manifest::{ArtifactRecord, ArtifactRole};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

const MAX_INVENTORY_DEPTH: usize = 16;
const MAX_INVENTORY_FILES: usize = 128;

pub(super) struct CandidateRoot {
    canonical: PathBuf,
}

#[derive(Debug)]
pub(super) struct ResolvedArtifact {
    pub role: ArtifactRole,
    pub relative: String,
    pub absolute: PathBuf,
    bytes: u64,
    sha256: String,
}

impl ResolvedArtifact {
    pub(super) fn record(&self) -> ArtifactRecord {
        ArtifactRecord {
            role: self.role,
            path: self.relative.clone(),
            bytes: self.bytes,
            sha256: self.sha256.clone(),
            public: self.role.is_public(),
        }
    }
}

impl CandidateRoot {
    pub(super) fn open(path: &Path) -> Result<Self> {
        let metadata = fs::symlink_metadata(path)
            .with_context(|| format!("inspect candidate directory {}", path.display()))?;
        if !metadata.is_dir() || is_link_or_reparse(&metadata) {
            bail!(
                "candidate directory must be a real directory, not a symlink or reparse point: {}",
                path.display()
            );
        }
        let canonical = fs::canonicalize(path)
            .with_context(|| format!("canonicalize candidate directory {}", path.display()))?;
        Ok(Self { canonical })
    }

    pub(super) fn resolve_artifact(
        &self,
        role: ArtifactRole,
        relative: &str,
    ) -> Result<ResolvedArtifact> {
        let resolved = self.resolve_existing_file(relative)?;
        let (sha256, bytes) = sha256_file(&resolved.absolute)?;
        Ok(ResolvedArtifact {
            role,
            relative: resolved.relative,
            absolute: resolved.absolute,
            bytes,
            sha256,
        })
    }

    pub(super) fn resolve_existing_file(&self, relative: &str) -> Result<ResolvedPath> {
        validate_relative_path(relative)?;
        let mut current = self.canonical.clone();
        for segment in relative.split('/') {
            current.push(segment);
            let metadata = fs::symlink_metadata(&current)
                .with_context(|| format!("inspect candidate path {}", current.display()))?;
            if is_link_or_reparse(&metadata) {
                bail!(
                    "candidate path contains a symlink or reparse point: {}",
                    current.display()
                );
            }
        }
        let metadata = fs::metadata(&current)
            .with_context(|| format!("inspect candidate file {}", current.display()))?;
        if !metadata.is_file() {
            bail!(
                "candidate artifact is not a regular file: {}",
                current.display()
            );
        }
        let canonical = fs::canonicalize(&current)
            .with_context(|| format!("canonicalize candidate file {}", current.display()))?;
        if !canonical.starts_with(&self.canonical) {
            bail!("candidate path escapes the candidate directory: {relative}");
        }
        Ok(ResolvedPath {
            relative: relative.to_owned(),
            absolute: canonical,
        })
    }

    pub(super) fn validate_inventory(&self, expected: &BTreeSet<String>) -> Result<()> {
        let actual = inventory(&self.canonical)?;
        let expected_directories = expected_directories(expected);
        if actual.files != *expected || actual.directories != expected_directories {
            let missing: Vec<_> = expected.difference(&actual.files).cloned().collect();
            let extra: Vec<_> = actual.files.difference(expected).cloned().collect();
            let missing_directories: Vec<_> = expected_directories
                .difference(&actual.directories)
                .cloned()
                .collect();
            let extra_directories: Vec<_> = actual
                .directories
                .difference(&expected_directories)
                .cloned()
                .collect();
            bail!(
                "release candidate inventory does not match allowlist; missing={missing:?}, extra={extra:?}, missing_directories={missing_directories:?}, extra_directories={extra_directories:?}"
            );
        }
        Ok(())
    }

    pub(super) fn write_generated(&self, relative: &str, bytes: &[u8]) -> Result<()> {
        validate_relative_path(relative)?;
        if relative.contains('/') {
            bail!("generated candidate control files must live at the candidate root");
        }
        let path = self.canonical.join(relative);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| {
                format!(
                    "create sealed candidate control file {}; candidate assembly is single-use",
                    path.display()
                )
            })?;
        let result = (|| {
            file.write_all(bytes)
                .with_context(|| format!("write {}", path.display()))?;
            file.sync_all()
                .with_context(|| format!("sync {}", path.display()))
        })();
        if let Err(error) = result {
            drop(file);
            return match fs::remove_file(&path) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(error.context(format!(
                    "also failed to remove partial control file {}: {cleanup}",
                    path.display()
                ))),
            };
        }
        Ok(())
    }

    pub(super) fn require_exact_file(&self, relative: &str, expected: &[u8]) -> Result<()> {
        let resolved = self.resolve_existing_file(relative)?;
        let metadata = fs::metadata(&resolved.absolute)
            .with_context(|| format!("inspect {}", resolved.absolute.display()))?;
        if metadata.len() != expected.len() as u64 {
            bail!("candidate control file has unexpected length: {relative}");
        }
        let actual = fs::read(&resolved.absolute)
            .with_context(|| format!("read {}", resolved.absolute.display()))?;
        if actual != expected {
            bail!("candidate control file does not match recomputed bytes: {relative}");
        }
        Ok(())
    }

    pub(super) fn read_bounded_control(&self, relative: &str, maximum: u64) -> Result<Vec<u8>> {
        let resolved = self.resolve_existing_file(relative)?;
        let metadata = fs::metadata(&resolved.absolute)
            .with_context(|| format!("inspect {}", resolved.absolute.display()))?;
        if metadata.len() > maximum {
            bail!("candidate control file exceeds {maximum} bytes: {relative}");
        }
        fs::read(&resolved.absolute)
            .with_context(|| format!("read {}", resolved.absolute.display()))
    }

    pub(super) fn remove_generated(&self, relative: &[&str]) -> Result<()> {
        let mut failures = Vec::new();
        for name in relative {
            if let Err(error) = fs::remove_file(self.canonical.join(name))
                && error.kind() != std::io::ErrorKind::NotFound
            {
                failures.push(format!("{name}: {error}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            bail!(
                "failed to roll back partial candidate control files: {}",
                failures.join(", ")
            )
        }
    }
}

pub(super) struct ResolvedPath {
    pub relative: String,
    pub absolute: PathBuf,
}

pub(super) fn validate_relative_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains('\\')
        || value.contains('\0')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.starts_with("//")
        || has_drive_prefix(value)
    {
        bail!("candidate path must be a canonical forward-slash relative path: {value:?}");
    }
    let mut segments = 0usize;
    for segment in value.split('/') {
        if segment.is_empty() || matches!(segment, "." | "..") {
            bail!("candidate path contains a non-canonical segment: {value:?}");
        }
        if segment.chars().any(char::is_control) {
            bail!("candidate path contains a control character: {value:?}");
        }
        segments += 1;
    }
    if segments > MAX_INVENTORY_DEPTH {
        bail!("candidate path exceeds maximum depth {MAX_INVENTORY_DEPTH}: {value:?}");
    }
    Ok(())
}

fn has_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

struct Inventory {
    files: BTreeSet<String>,
    directories: BTreeSet<String>,
}

fn inventory(root: &Path) -> Result<Inventory> {
    let mut files = BTreeSet::new();
    let mut directories = BTreeSet::new();
    let mut entries_seen = 0usize;
    let mut pending = vec![(root.to_path_buf(), String::new(), 0usize)];
    while let Some((directory, prefix, depth)) = pending.pop() {
        if depth > MAX_INVENTORY_DEPTH {
            bail!("candidate inventory exceeds maximum depth {MAX_INVENTORY_DEPTH}");
        }
        let mut entries: Vec<_> = fs::read_dir(&directory)
            .with_context(|| format!("read candidate directory {}", directory.display()))?
            .collect::<std::result::Result<_, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            entries_seen += 1;
            if entries_seen > MAX_INVENTORY_FILES {
                bail!("candidate inventory exceeds {MAX_INVENTORY_FILES} entries");
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.contains('/') || name.contains('\\') || name.chars().any(char::is_control) {
                bail!("candidate inventory contains a non-portable file name: {name:?}");
            }
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            validate_relative_path(&relative)?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if is_link_or_reparse(&metadata) {
                bail!("candidate inventory contains a symlink or reparse point: {relative}");
            }
            if metadata.is_dir() {
                directories.insert(relative.clone());
                pending.push((entry.path(), relative, depth + 1));
            } else if metadata.is_file() {
                files.insert(relative);
            } else {
                bail!("candidate inventory contains a non-regular entry: {relative}");
            }
        }
    }
    Ok(Inventory { files, directories })
}

fn expected_directories(files: &BTreeSet<String>) -> BTreeSet<String> {
    let mut directories = BTreeSet::new();
    for file in files {
        let mut prefix = String::new();
        let mut segments = file.split('/').peekable();
        while let Some(segment) = segments.next() {
            if segments.peek().is_none() {
                break;
            }
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(segment);
            directories.insert(prefix.clone());
        }
    }
    directories
}

fn sha256_file(path: &Path) -> Result<(String, u64)> {
    let file = File::open(path).with_context(|| format!("open {} for hashing", path.display()))?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut bytes = 0u64;
    loop {
        let count = reader
            .read(&mut buffer)
            .with_context(|| format!("hash {}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes = bytes
            .checked_add(count as u64)
            .ok_or_else(|| anyhow::anyhow!("artifact size overflow for {}", path.display()))?;
    }
    Ok((format!("{:x}", hasher.finalize()), bytes))
}

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_link_or_reparse(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
