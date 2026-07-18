use std::collections::BTreeMap;

use crate::remote_projection::RemoteProjectionProvider;

use super::super::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProviderAdapter,
    RemoteProjectionProviderError, RemoteProjectionPushOutcome, RemoteProjectionPushRequest,
    validate_unique_paths,
};

#[derive(Debug, Clone)]
pub(super) struct FakeRemoteProjectionProvider {
    provider: RemoteProjectionProvider,
    remotes: BTreeMap<String, Vec<RemoteProjectionFile>>,
}

impl FakeRemoteProjectionProvider {
    pub(super) fn new(provider: RemoteProjectionProvider) -> Self {
        Self {
            provider,
            remotes: BTreeMap::new(),
        }
    }

    pub(super) fn remote_files(&self, locator: &str) -> Option<&[RemoteProjectionFile]> {
        self.remotes.get(locator.trim()).map(Vec::as_slice)
    }
}

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
}

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
