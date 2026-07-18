//! plan_ref:
//!   - 06_backup#remote-import-resource-contract
//!   - 06_backup#remote-projection-transport-contract
//!
//! Provider-neutral path admission and bounded discovery accumulation.

use deve_core::remote_projection::{RemoteProjectionFile, RemoteProjectionProviderError};
use deve_core::utils::path::validate_projection_repo_child_path;
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

pub(crate) const MAX_SOURCE_FILES: usize = 2_048;
pub(crate) const MAX_SOURCE_PATH_BYTES: usize = 1_024;
pub(crate) const MAX_SOURCE_TOTAL_PATH_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct NormalizedRemotePath(String);

impl NormalizedRemotePath {
    pub(crate) fn new(path: impl AsRef<str>) -> Result<Self, RemoteProjectionProviderError> {
        let original = path.as_ref();
        if original.nfc().collect::<String>() != original {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        }
        validate_projection_repo_child_path(original)
            .map_err(|_| RemoteProjectionProviderError::InvalidProjectionPath)?;
        validate_host_canonical_segments(original)?;
        let validated = RemoteProjectionFile::new(original, Vec::new())?;
        let path = validated.path();
        if path != original {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        }
        if path.len() > MAX_SOURCE_PATH_BYTES {
            return Err(path_budget_error(format!(
                "remote source path exceeds {MAX_SOURCE_PATH_BYTES} bytes"
            )));
        }
        Ok(Self(path.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(super) struct NormalizedDiscoveryPath(String);

impl NormalizedDiscoveryPath {
    pub(super) fn new(path: impl AsRef<str>) -> Result<Self, RemoteProjectionProviderError> {
        let path = path.as_ref();
        if path.nfc().collect::<String>() != path {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        }
        validate_projection_repo_child_path(path)
            .map_err(|_| RemoteProjectionProviderError::InvalidProjectionPath)?;
        validate_host_canonical_segments(path)?;
        if path.len() > MAX_SOURCE_PATH_BYTES {
            return Err(path_budget_error(format!(
                "remote source path exceeds {MAX_SOURCE_PATH_BYTES} bytes"
            )));
        }
        Ok(Self(path.to_string()))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

// This is deliberately a transport-local pre-admission mirror of the Remote
// Import artifact policy. The core sink revalidates the same host constraints
// before publication; keeping this check here prevents invalid provider paths
// from causing payload I/O without widening the crate boundary.
fn validate_host_canonical_segments(path: &str) -> Result<(), RemoteProjectionProviderError> {
    for segment in path.split('/') {
        if segment.chars().any(|character| {
            character.is_control() || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
        }) {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        }
        let device_stem = segment.split('.').next().unwrap_or(segment);
        let upper = device_stem.to_ascii_uppercase();
        let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || upper.strip_prefix("COM").is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
            || upper.strip_prefix("LPT").is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            });
        if reserved {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        }
    }
    Ok(())
}

pub(super) struct RemotePathBudget {
    label: &'static str,
    total_bytes: usize,
}

impl RemotePathBudget {
    pub(super) fn new(label: &'static str) -> Self {
        Self {
            label,
            total_bytes: 0,
        }
    }

    pub(super) fn observe(&mut self, path: &str) -> Result<(), RemoteProjectionProviderError> {
        let next_total = self
            .total_bytes
            .checked_add(path.len())
            .ok_or_else(|| path_budget_error(format!("{} path bytes overflow", self.label)))?;
        if next_total > MAX_SOURCE_TOTAL_PATH_BYTES {
            return Err(path_budget_error(format!(
                "{} exceeds total path byte budget of {MAX_SOURCE_TOTAL_PATH_BYTES}",
                self.label
            )));
        }
        self.total_bytes = next_total;
        Ok(())
    }
}

pub(super) struct DiscoveredRemotePaths {
    label: &'static str,
    paths: BTreeSet<NormalizedRemotePath>,
    casefold_paths: BTreeSet<String>,
    path_budget: RemotePathBudget,
}

impl DiscoveredRemotePaths {
    pub(super) fn new(label: &'static str) -> Self {
        Self {
            label,
            paths: BTreeSet::new(),
            casefold_paths: BTreeSet::new(),
            path_budget: RemotePathBudget::new(label),
        }
    }

    pub(super) fn insert(
        &mut self,
        path: NormalizedRemotePath,
    ) -> Result<(), RemoteProjectionProviderError> {
        let casefold_path = path.as_str().to_lowercase();
        if self.paths.contains(&path) || self.casefold_paths.contains(&casefold_path) {
            return Err(RemoteProjectionProviderError::DuplicateProjectionPath);
        }
        if self.paths.len() == MAX_SOURCE_FILES {
            return Err(path_budget_error(format!(
                "{} exceeds file budget of {MAX_SOURCE_FILES}",
                self.label
            )));
        }
        self.path_budget.observe(path.as_str())?;
        self.casefold_paths.insert(casefold_path);
        self.paths.insert(path);
        Ok(())
    }

    pub(super) fn into_sorted_vec(self) -> Vec<NormalizedRemotePath> {
        self.paths.into_iter().collect()
    }
}

fn path_budget_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_remote_path_rejects_more_than_1024_bytes() {
        let error = NormalizedRemotePath::new(format!("{}.md", "a".repeat(1_022)))
            .expect_err("1025-byte path");
        assert!(error.to_string().contains("exceeds 1024 bytes"));
    }

    #[test]
    fn normalized_remote_path_rejects_non_canonical_spelling() {
        for path in [
            " notes/a.md",
            "notes/a.md ",
            "notes\\a.md",
            "notes/e\u{301}.md",
        ] {
            assert_eq!(
                NormalizedRemotePath::new(path),
                Err(RemoteProjectionProviderError::InvalidProjectionPath),
                "{path:?}"
            );
        }
    }

    #[test]
    fn normalized_remote_path_rejects_host_reserved_names_and_characters() {
        for path in ["CON.md", "notes/Lpt9.markdown", "bad?.md", "bad\u{7f}.md"] {
            assert_eq!(
                NormalizedRemotePath::new(path),
                Err(RemoteProjectionProviderError::InvalidProjectionPath),
                "{path:?}"
            );
        }
    }

    #[test]
    fn remote_path_budget_rejects_more_than_two_mib() {
        let path = NormalizedRemotePath::new(format!("{}.md", "a".repeat(1_021))).expect("path");
        let mut budget = RemotePathBudget::new("test discovery");
        for _ in 0..MAX_SOURCE_FILES {
            budget.observe(path.as_str()).expect("within budget");
        }
        let error = budget
            .observe(path.as_str())
            .expect_err("total path budget");
        assert!(error.to_string().contains("total path byte budget"));
    }
}
