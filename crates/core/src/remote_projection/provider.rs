//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!
//! Provider adapter boundary for Markdown projection transport.

#[cfg(test)]
mod tests;

#[cfg(test)]
use std::collections::BTreeMap;

use crate::utils::path::to_forward_slash;

use super::{
    RemoteProjectionDirection, RemoteProjectionPlanInput, RemoteProjectionProvider,
    plan_remote_projection_transport,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProjectionFile {
    path: String,
    content: Vec<u8>,
}

impl RemoteProjectionFile {
    pub fn new(
        path: impl AsRef<str>,
        content: impl Into<Vec<u8>>,
    ) -> Result<Self, RemoteProjectionProviderError> {
        let path = normalize_projection_file_path(path.as_ref())?;
        Ok(Self {
            path,
            content: content.into(),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProjectionPushRequest {
    provider: RemoteProjectionProvider,
    locator: String,
    files: Vec<RemoteProjectionFile>,
}

impl RemoteProjectionPushRequest {
    pub fn new(
        provider: RemoteProjectionProvider,
        locator: impl Into<String>,
        files: Vec<RemoteProjectionFile>,
    ) -> Result<Self, RemoteProjectionProviderError> {
        let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
            provider,
            direction: RemoteProjectionDirection::Push,
            locator: locator.into(),
        })?;
        validate_unique_paths(&files)?;
        Ok(Self {
            provider: plan.provider,
            locator: plan.locator,
            files,
        })
    }

    pub fn provider(&self) -> RemoteProjectionProvider {
        self.provider
    }

    pub fn locator(&self) -> &str {
        &self.locator
    }

    pub fn files(&self) -> &[RemoteProjectionFile] {
        &self.files
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProjectionPullRequest {
    provider: RemoteProjectionProvider,
    locator: String,
}

impl RemoteProjectionPullRequest {
    pub fn new(
        provider: RemoteProjectionProvider,
        locator: impl Into<String>,
    ) -> Result<Self, RemoteProjectionProviderError> {
        let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
            provider,
            direction: RemoteProjectionDirection::Pull,
            locator: locator.into(),
        })?;
        Ok(Self {
            provider: plan.provider,
            locator: plan.locator,
        })
    }

    pub fn provider(&self) -> RemoteProjectionProvider {
        self.provider
    }

    pub fn locator(&self) -> &str {
        &self.locator
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProjectionAuthorityEffects {
    pub writes_ledger: bool,
    pub writes_source_control_staging: bool,
    pub writes_commit_anchor: bool,
    pub writes_git_main_mirror: bool,
    pub confirms_external_changes: bool,
}

impl RemoteProjectionAuthorityEffects {
    pub fn projection_transport() -> Self {
        Self {
            writes_ledger: false,
            writes_source_control_staging: false,
            writes_commit_anchor: false,
            writes_git_main_mirror: false,
            confirms_external_changes: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProjectionPushOutcome {
    pub uploaded_files: usize,
    pub effects: RemoteProjectionAuthorityEffects,
    pub provider_metadata_is_diagnostic_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProjectionPullOutcome {
    pub files: Vec<RemoteProjectionFile>,
    pub effects: RemoteProjectionAuthorityEffects,
    pub overwrites_projection_workspace: bool,
    pub external_changes_confirmation_required: bool,
    pub provider_metadata_is_diagnostic_only: bool,
}

pub trait RemoteProjectionProviderAdapter {
    fn provider(&self) -> RemoteProjectionProvider;

    fn push(
        &mut self,
        request: RemoteProjectionPushRequest,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError>;

    fn pull(
        &self,
        request: RemoteProjectionPullRequest,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RemoteProjectionProviderError {
    #[error(transparent)]
    AdmissionRejected(#[from] super::RemoteProjectionError),
    #[error("remote projection provider mismatch")]
    ProviderMismatch,
    #[error("remote projection file path must be a relative markdown path")]
    InvalidProjectionPath,
    #[error("remote projection file path targets internal state")]
    InternalStatePath,
    #[error("remote projection file path is duplicated")]
    DuplicateProjectionPath,
    #[error("remote projection locator has no fake remote content")]
    MissingFakeRemote,
    #[error("remote projection provider I/O failed: {0}")]
    ProviderIo(String),
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct FakeRemoteProjectionProvider {
    provider: RemoteProjectionProvider,
    remotes: BTreeMap<String, Vec<RemoteProjectionFile>>,
}

#[cfg(test)]
impl FakeRemoteProjectionProvider {
    fn new(provider: RemoteProjectionProvider) -> Self {
        Self {
            provider,
            remotes: BTreeMap::new(),
        }
    }

    fn seed_remote(
        &mut self,
        locator: impl Into<String>,
        files: Vec<RemoteProjectionFile>,
    ) -> Result<(), RemoteProjectionProviderError> {
        let request = RemoteProjectionPullRequest::new(self.provider, locator)?;
        validate_unique_paths(&files)?;
        self.remotes.insert(request.locator, files);
        Ok(())
    }

    fn remote_files(&self, locator: &str) -> Option<&[RemoteProjectionFile]> {
        self.remotes.get(locator.trim()).map(Vec::as_slice)
    }
}

#[cfg(test)]
impl RemoteProjectionProviderAdapter for FakeRemoteProjectionProvider {
    fn provider(&self) -> RemoteProjectionProvider {
        self.provider
    }

    fn push(
        &mut self,
        request: RemoteProjectionPushRequest,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        validate_provider(self.provider, request.provider)?;
        validate_unique_paths(&request.files)?;
        let uploaded_files = request.files.len();
        self.remotes.insert(request.locator, request.files);
        Ok(RemoteProjectionPushOutcome {
            uploaded_files,
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: true,
        })
    }

    fn pull(
        &self,
        request: RemoteProjectionPullRequest,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        validate_provider(self.provider, request.provider)?;
        let files = self
            .remotes
            .get(&request.locator)
            .ok_or(RemoteProjectionProviderError::MissingFakeRemote)?
            .clone();
        Ok(RemoteProjectionPullOutcome {
            files,
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            overwrites_projection_workspace: true,
            external_changes_confirmation_required: true,
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

#[cfg(test)]
fn validate_provider(
    expected: RemoteProjectionProvider,
    actual: RemoteProjectionProvider,
) -> Result<(), RemoteProjectionProviderError> {
    if expected == actual {
        Ok(())
    } else {
        Err(RemoteProjectionProviderError::ProviderMismatch)
    }
}

fn validate_unique_paths(
    files: &[RemoteProjectionFile],
) -> Result<(), RemoteProjectionProviderError> {
    let mut paths = std::collections::BTreeSet::new();
    for file in files {
        if !paths.insert(file.path()) {
            return Err(RemoteProjectionProviderError::DuplicateProjectionPath);
        }
    }
    Ok(())
}

fn normalize_projection_file_path(path: &str) -> Result<String, RemoteProjectionProviderError> {
    let path = to_forward_slash(path.trim());
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(':')
        || !(path.ends_with(".md") || path.ends_with(".markdown"))
    {
        return Err(RemoteProjectionProviderError::InvalidProjectionPath);
    }

    for segment in path.split('/') {
        if matches!(segment, "" | "." | "..") {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        }
        if matches!(
            segment,
            ".git" | ".notegit" | "ledger" | "snapshot" | "snapshots" | "staging"
        ) {
            return Err(RemoteProjectionProviderError::InternalStatePath);
        }
    }

    Ok(path)
}
