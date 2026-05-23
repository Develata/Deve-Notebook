//! plan_ref:
//!   - 18_backup#backup-locator-contract

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupProviderKind {
    WebDavHttps,
    S3,
    S3CompatibleHttps,
}

impl BackupProviderKind {
    pub fn protocol(self) -> &'static str {
        match self {
            Self::WebDavHttps => "webdav+https",
            Self::S3 => "s3",
            Self::S3CompatibleHttps => "s3+https",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupLocator {
    pub provider: BackupProviderKind,
    pub endpoint: Option<String>,
    pub namespace: String,
    pub repo_root_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchBackupLocator {
    pub root: BackupLocator,
    pub writer_identity: String,
    pub branch_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupLocatorError {
    #[error("unsupported backup locator scheme")]
    UnsupportedScheme,
    #[error("backup locator endpoint is missing")]
    MissingEndpoint,
    #[error("backup locator namespace is missing")]
    MissingNamespace,
    #[error("backup locator repo root path is missing")]
    MissingRepoRootPath,
    #[error("backup locator must not contain credentials, tokens, query, or fragment data")]
    SecretMaterialForbidden,
    #[error("backup locator contains an unsafe remote path segment: {0}")]
    UnsafeRemotePath(String),
    #[error("backup branch writer identity is not a safe path segment: {0}")]
    UnsafeWriterIdentity(String),
}

impl BackupLocator {
    pub fn parse(input: &str) -> Result<Self, BackupLocatorError> {
        let input = input.trim();
        if let Some(rest) = input.strip_prefix("webdav+https://") {
            return parse_webdav_https(rest);
        }
        if let Some(rest) = input.strip_prefix("s3+https://") {
            return parse_s3_compatible_https(rest);
        }
        if let Some(rest) = input.strip_prefix("s3://") {
            return parse_s3(rest);
        }
        Err(BackupLocatorError::UnsupportedScheme)
    }

    pub fn branch_locator(
        &self,
        writer_identity: &str,
    ) -> Result<BranchBackupLocator, BackupLocatorError> {
        let writer_identity = safe_writer_identity(writer_identity)?;
        let branch_path = format!("{}/branches/{}", self.repo_root_path, writer_identity);
        Ok(BranchBackupLocator {
            root: self.clone(),
            writer_identity,
            branch_path,
        })
    }
}

impl BranchBackupLocator {
    pub fn branch_manifest_path(&self) -> String {
        format!("{}/branch.manifest.enc", self.branch_path)
    }

    pub fn pack_prefix(&self) -> String {
        format!("{}/packs", self.branch_path)
    }
}

fn parse_webdav_https(rest: &str) -> Result<BackupLocator, BackupLocatorError> {
    let (authority, path) = authority_and_path(rest)?;
    let repo_root_path = normalize_remote_path(path)?;
    Ok(BackupLocator {
        provider: BackupProviderKind::WebDavHttps,
        endpoint: Some(format!("https://{authority}")),
        namespace: authority.to_string(),
        repo_root_path,
    })
}

fn parse_s3_compatible_https(rest: &str) -> Result<BackupLocator, BackupLocatorError> {
    let (authority, path) = authority_and_path(rest)?;
    let (bucket, repo_root) = first_path_segment(path)?;
    let namespace = safe_namespace(bucket)?;
    let repo_root_path = normalize_remote_path(repo_root)?;
    Ok(BackupLocator {
        provider: BackupProviderKind::S3CompatibleHttps,
        endpoint: Some(format!("https://{authority}")),
        namespace,
        repo_root_path,
    })
}

fn parse_s3(rest: &str) -> Result<BackupLocator, BackupLocatorError> {
    let (bucket, path) = authority_and_path(rest)?;
    let namespace = safe_namespace(bucket)?;
    let repo_root_path = normalize_remote_path(path)?;
    Ok(BackupLocator {
        provider: BackupProviderKind::S3,
        endpoint: None,
        namespace,
        repo_root_path,
    })
}

fn authority_and_path(input: &str) -> Result<(&str, &str), BackupLocatorError> {
    reject_secret_material(input)?;
    let (authority, path) = input
        .split_once('/')
        .ok_or(BackupLocatorError::MissingRepoRootPath)?;
    if authority.is_empty() {
        return Err(BackupLocatorError::MissingEndpoint);
    }
    if authority.contains('@') {
        return Err(BackupLocatorError::SecretMaterialForbidden);
    }
    if authority
        .chars()
        .any(|ch| ch.is_ascii_control() || ch == ' ')
    {
        return Err(BackupLocatorError::MissingEndpoint);
    }
    Ok((authority, path))
}

fn first_path_segment(input: &str) -> Result<(&str, &str), BackupLocatorError> {
    let mut segments = input.splitn(2, '/');
    let first = segments.next().unwrap_or_default();
    let rest = segments.next().unwrap_or_default();
    if first.is_empty() {
        return Err(BackupLocatorError::MissingNamespace);
    }
    Ok((first, rest))
}

fn reject_secret_material(input: &str) -> Result<(), BackupLocatorError> {
    if input.contains('?') || input.contains('#') {
        return Err(BackupLocatorError::SecretMaterialForbidden);
    }
    if input.contains('\0') {
        return Err(BackupLocatorError::SecretMaterialForbidden);
    }
    Ok(())
}

fn safe_namespace(input: &str) -> Result<String, BackupLocatorError> {
    if input.is_empty() {
        return Err(BackupLocatorError::MissingNamespace);
    }
    if input.contains('@') || input.contains(':') || input.contains('\\') || input.contains('/') {
        return Err(BackupLocatorError::SecretMaterialForbidden);
    }
    if input.chars().any(|ch| ch.is_ascii_control() || ch == ' ') {
        return Err(BackupLocatorError::MissingNamespace);
    }
    Ok(input.to_string())
}

pub(crate) fn normalize_remote_path(input: &str) -> Result<String, BackupLocatorError> {
    reject_secret_material(input)?;
    if input.starts_with('/') {
        return Err(BackupLocatorError::UnsafeRemotePath(String::new()));
    }
    let trimmed = input.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(BackupLocatorError::MissingRepoRootPath);
    }
    let mut out = Vec::new();
    for segment in trimmed.split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('\\')
            || segment.chars().any(|ch| ch.is_ascii_control() || ch == ' ')
        {
            return Err(BackupLocatorError::UnsafeRemotePath(segment.to_string()));
        }
        out.push(segment);
    }
    Ok(out.join("/"))
}

pub(crate) fn safe_writer_identity(input: &str) -> Result<String, BackupLocatorError> {
    if input.trim() != input {
        return Err(BackupLocatorError::UnsafeWriterIdentity(input.to_string()));
    }
    let value = input;
    if value.is_empty() || value == "." || value == ".." {
        return Err(BackupLocatorError::UnsafeWriterIdentity(value.to_string()));
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(BackupLocatorError::UnsafeWriterIdentity(value.to_string()));
    }
    Ok(value.to_string())
}
