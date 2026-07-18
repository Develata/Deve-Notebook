//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!
//! Provider-neutral transport contracts. Push and source acquisition are
//! deliberately separate interfaces even when they share one HTTP adapter.

use super::path_set::NormalizedRemotePath;
#[cfg(test)]
use deve_core::remote_projection::RemoteProjectionFile;
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionDirection, RemoteProjectionPlanInput,
    RemoteProjectionProvider, RemoteProjectionProviderError, RemoteProjectionPushOutcome,
    plan_remote_projection_transport,
};
use std::fmt;
#[cfg(test)]
use std::io;
use std::io::Read;

pub(crate) type ProjectionPushVisitor<'a> =
    dyn FnMut(&str, Vec<u8>) -> Result<(), RemoteProjectionProviderError> + 'a;

pub(crate) trait ProjectionPushSource {
    fn file_count(&self) -> usize;

    fn visit(
        &self,
        visitor: &mut ProjectionPushVisitor<'_>,
    ) -> Result<(), RemoteProjectionProviderError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportCapability {
    Push,
    SourceAcquisition,
}

impl TransportCapability {
    pub(super) fn admission_direction(self) -> RemoteProjectionDirection {
        match self {
            Self::Push => RemoteProjectionDirection::Push,
            Self::SourceAcquisition => RemoteProjectionDirection::Pull,
        }
    }

    pub(super) fn profile_name(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::SourceAcquisition => "source-acquisition",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceAcquisitionRequest {
    provider: RemoteProjectionProvider,
    locator: String,
}

impl SourceAcquisitionRequest {
    pub(crate) fn new(
        provider: RemoteProjectionProvider,
        locator: impl Into<String>,
    ) -> Result<Self, RemoteProjectionProviderError> {
        // The public protocol keeps `Pull` until the B4 cutover. Inside the
        // host runtime it is only an admission selector; no pull/workspace
        // semantics cross this source-acquisition contract.
        let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
            provider,
            direction: TransportCapability::SourceAcquisition.admission_direction(),
            locator: locator.into(),
        })?;
        Ok(Self {
            provider: plan.provider,
            locator: plan.locator,
        })
    }

    pub(crate) fn provider(&self) -> RemoteProjectionProvider {
        self.provider
    }

    pub(crate) fn locator(&self) -> &str {
        &self.locator
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceAcquisitionOutcome {
    pub(crate) files: usize,
    pub(crate) bytes: usize,
}

pub(crate) trait RemoteSourceSink {
    type Error;

    fn capture(
        &mut self,
        path: &NormalizedRemotePath,
        body: &mut dyn Read,
    ) -> Result<(), Self::Error>;
}

pub(crate) enum SourceAcquisitionError<E> {
    Transport(RemoteProjectionProviderError),
    Sink(E),
}

impl<E: fmt::Debug> fmt::Debug for SourceAcquisitionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => formatter.debug_tuple("Transport").field(error).finish(),
            Self::Sink(error) => formatter.debug_tuple("Sink").field(error).finish(),
        }
    }
}

impl<E: fmt::Display> fmt::Display for SourceAcquisitionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => error.fmt(formatter),
            Self::Sink(error) => write!(formatter, "remote source sink failed: {error}"),
        }
    }
}

impl<E> From<RemoteProjectionProviderError> for SourceAcquisitionError<E> {
    fn from(error: RemoteProjectionProviderError) -> Self {
        Self::Transport(error)
    }
}

pub(crate) trait RemoteSourceAcquisition {
    fn provider(&self) -> RemoteProjectionProvider;

    fn acquire<S: RemoteSourceSink>(
        &self,
        request: SourceAcquisitionRequest,
        sink: &mut S,
    ) -> Result<SourceAcquisitionOutcome, SourceAcquisitionError<S::Error>>;
}

pub(crate) fn ensure_projection_transport_push_outcome_contract(
    outcome: &RemoteProjectionPushOutcome,
) -> anyhow::Result<()> {
    ensure_projection_transport_effects_absent(&outcome.effects)?;
    if outcome.provider_metadata_is_diagnostic_only {
        Ok(())
    } else {
        Err(super::provider_io_not_ready(
            "provider outcome violates remote projection transport contract: provider metadata must be diagnostic-only",
        ))
    }
}

fn ensure_projection_transport_effects_absent(
    effects: &RemoteProjectionAuthorityEffects,
) -> anyhow::Result<()> {
    if effects.writes_ledger
        || effects.writes_source_control_staging
        || effects.writes_commit_anchor
        || effects.writes_git_main_mirror
        || effects.confirms_external_changes
    {
        return Err(super::provider_io_not_ready(format!(
            "provider outcome violates remote projection transport contract: authority effects must be absent \
             (writes_ledger={}, writes_source_control_staging={}, writes_commit_anchor={}, writes_git_main_mirror={}, confirms_external_changes={})",
            effects.writes_ledger,
            effects.writes_source_control_staging,
            effects.writes_commit_anchor,
            effects.writes_git_main_mirror,
            effects.confirms_external_changes,
        )));
    }
    Ok(())
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct TestCollectingSourceSink {
    pub(crate) files: Vec<RemoteProjectionFile>,
}

#[cfg(test)]
impl RemoteSourceSink for TestCollectingSourceSink {
    type Error = io::Error;

    fn capture(
        &mut self,
        path: &NormalizedRemotePath,
        body: &mut dyn Read,
    ) -> Result<(), Self::Error> {
        let mut content = Vec::new();
        body.read_to_end(&mut content)?;
        self.files.push(
            RemoteProjectionFile::new(path.as_str(), content)
                .expect("normalized path remains valid"),
        );
        Ok(())
    }
}
