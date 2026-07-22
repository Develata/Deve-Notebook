//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 07_network#remote-import-wire-contract
//!   - 08_auth#local-cli-proxy-authority
//!
//! Host coordinator for provider acquisition and the narrow `deve_core`
//! Remote Import facade. It owns no Ledger tables or workspace decisions.

mod provider_tasks;
mod removal;
mod source_binding;

use crate::remote_projection_transport::{
    NormalizedRemotePath, RemoteSourceAcquisition, RemoteSourceSink, SourceAcquisitionError,
    SourceAcquisitionRequest,
};
use deve_core::ledger::{CatalogMembershipError, CatalogMembershipRuntime, RepoManager};
use deve_core::models::RepoId;
use deve_core::protocol::RemoteProjectionProvider;
use deve_core::remote_import::{
    RemoteImportApplyView, RemoteImportBinding, RemoteImportCandidatePage,
    RemoteImportCandidateRevision, RemoteImportDiffView, RemoteImportEntryId,
    RemoteImportPageCursor, RemoteImportRepairPlan, RemoteImportResult, RemoteImportService,
    RemoteImportSessionId, RemoteImportSessionView,
};
use deve_core::sync::SyncManager;
pub(crate) use provider_tasks::ProviderQuiesceToken;
use provider_tasks::{ProviderTaskError, ProviderTaskLease, ProviderTaskRuntime};
use source_binding::{ResolvedRemoteSource, canonical_binding_material, infer_provider};
use std::collections::HashSet;
use std::io::Read;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) enum RemoteImportHostError {
    Core(deve_core::remote_import::RemoteImportError),
    Locator(String),
    Provider(String),
    ProviderCleanup { primary: String, cleanup: String },
    ProviderBusy,
    RepoMembership(CatalogMembershipError),
    ApplyBusy,
    Coordination,
}

impl From<deve_core::remote_import::RemoteImportError> for RemoteImportHostError {
    fn from(error: deve_core::remote_import::RemoteImportError) -> Self {
        Self::Core(error)
    }
}

impl From<ProviderTaskError> for RemoteImportHostError {
    fn from(error: ProviderTaskError) -> Self {
        match error {
            ProviderTaskError::Busy => Self::ProviderBusy,
            ProviderTaskError::Membership(error) => Self::RepoMembership(error),
            ProviderTaskError::Coordination => Self::Coordination,
        }
    }
}

impl std::fmt::Display for RemoteImportHostError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::Locator(detail) => {
                write!(formatter, "Remote Import locator is invalid: {detail}")
            }
            Self::Provider(detail) => {
                write!(formatter, "Remote Import provider is unavailable: {detail}")
            }
            Self::ProviderCleanup { primary, cleanup } => write!(
                formatter,
                "Remote Import provider failed and session cleanup also failed: primary={primary}; cleanup={cleanup}"
            ),
            Self::ProviderBusy => write!(
                formatter,
                "Remote Import provider task is busy or quiescing for this repo"
            ),
            Self::RepoMembership(error) => {
                write!(formatter, "Remote Import repo membership is stale: {error}")
            }
            Self::ApplyBusy => write!(
                formatter,
                "Remote Import apply is already running for this repo"
            ),
            Self::Coordination => {
                write!(formatter, "Remote Import coordinator state is unavailable")
            }
        }
    }
}

impl std::error::Error for RemoteImportHostError {}

pub(crate) struct RemoteImportCoordinator {
    repo: Arc<RepoManager>,
    sync: Arc<SyncManager>,
    membership: CatalogMembershipRuntime,
    applying: Mutex<HashSet<RepoId>>,
    providers: ProviderTaskRuntime,
}

impl RemoteImportCoordinator {
    pub(crate) fn new(
        repo: Arc<RepoManager>,
        sync: Arc<SyncManager>,
        membership: CatalogMembershipRuntime,
    ) -> Self {
        Self {
            repo,
            sync,
            membership,
            applying: Mutex::new(HashSet::new()),
            providers: ProviderTaskRuntime::default(),
        }
    }

    pub(crate) fn prepare(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        provider: RemoteProjectionProvider,
    ) -> Result<RemoteImportSessionView, RemoteImportHostError> {
        let membership = self
            .membership
            .issue(repo_id)
            .map_err(RemoteImportHostError::RepoMembership)?;
        let mut provider_lease = self.acquire_provider(repo_id)?;
        let source = self.resolve_source(repo_name, Some(provider))?;
        let service = RemoteImportService::open(&self.repo, repo_id)?;
        let mut capture = service.begin_prepare(
            &self.repo,
            repo_name,
            &source.source_binding,
            &source.locator_binding,
        )?;
        let session_id = capture.session_id();
        provider_lease.bind_session(session_id)?;
        let request = SourceAcquisitionRequest::new(source.provider, source.locator.clone())
            .map_err(|error| RemoteImportHostError::Locator(error.to_string()))?;
        let acquisition = match source.provider {
            RemoteProjectionProvider::WebDav => {
                let provider =
                    crate::remote_projection_transport::webdav::WebDavProjectionProvider::new()
                        .map_err(|error| RemoteImportHostError::Provider(error.to_string()))?;
                provider.acquire(request, &mut CaptureBridge(&mut capture))
            }
            RemoteProjectionProvider::S3 => source
                .s3_provider
                .ok_or(RemoteImportHostError::Coordination)?
                .acquire(request, &mut CaptureBridge(&mut capture)),
        };
        match acquisition {
            Ok(outcome) => {
                tracing::info!(
                    provider = source.provider.as_str(),
                    files = outcome.files,
                    bytes = outcome.bytes,
                    "Remote Import immutable source capture completed"
                );
                if let Err(error) =
                    provider_lease.revalidate_completion(&self.membership, &membership, session_id)
                {
                    let error = RemoteImportHostError::from(error);
                    return match capture.abort_source() {
                        Ok(_) => Err(error),
                        Err(cleanup) => Err(RemoteImportHostError::ProviderCleanup {
                            primary: error.to_string(),
                            cleanup: cleanup.to_string(),
                        }),
                    };
                }
                capture.finish().map_err(Into::into)
            }
            Err(SourceAcquisitionError::Sink(error)) => Err(error.into()),
            Err(SourceAcquisitionError::Transport(primary)) => match capture.abort_source() {
                Ok(_) => Err(RemoteImportHostError::Provider(primary.to_string())),
                Err(cleanup) => Err(RemoteImportHostError::ProviderCleanup {
                    primary: primary.to_string(),
                    cleanup: cleanup.to_string(),
                }),
            },
        }
    }

    pub(crate) fn list(
        &self,
        repo_id: RepoId,
    ) -> Result<Vec<RemoteImportSessionView>, RemoteImportHostError> {
        Ok(RemoteImportService::open(&self.repo, repo_id)?.list()?)
    }

    pub(crate) fn show(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    ) -> Result<RemoteImportSessionView, RemoteImportHostError> {
        let source = self.resolve_source(repo_name, None)?;
        Ok(RemoteImportService::open(&self.repo, repo_id)?.show(
            &self.repo,
            repo_name,
            session_id,
            revision,
            &source.locator_binding,
        )?)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn page(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        cursor: Option<&RemoteImportPageCursor>,
        limit: usize,
    ) -> Result<RemoteImportCandidatePage, RemoteImportHostError> {
        let source = self.resolve_source(repo_name, None)?;
        Ok(RemoteImportService::open(&self.repo, repo_id)?.page(
            &self.repo,
            repo_name,
            session_id,
            revision,
            cursor,
            limit,
            &source.locator_binding,
        )?)
    }

    pub(crate) fn diff(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        entry_id: &RemoteImportEntryId,
    ) -> Result<RemoteImportDiffView, RemoteImportHostError> {
        Ok(RemoteImportService::open(&self.repo, repo_id)?
            .diff(&self.repo, repo_name, session_id, revision, entry_id)?)
    }

    pub(crate) fn refresh(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    ) -> Result<RemoteImportSessionView, RemoteImportHostError> {
        let source = self.resolve_source(repo_name, None)?;
        Ok(RemoteImportService::open(&self.repo, repo_id)?.refresh(
            &self.repo,
            repo_name,
            session_id,
            revision,
            &source.source_binding,
            &source.locator_binding,
        )?)
    }

    pub(crate) fn apply(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        request_id: Uuid,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    ) -> Result<RemoteImportApplyView, RemoteImportHostError> {
        let _lease = self.acquire_apply(repo_id)?;
        let service = RemoteImportService::open(&self.repo, repo_id)?;
        let exact_replay =
            service.is_exact_apply_replay(&self.repo, request_id, session_id, revision)?;
        let source = if exact_replay {
            None
        } else {
            Some(self.resolve_source(repo_name, None)?)
        };
        Ok(service.apply(
            &self.repo,
            &self.sync,
            repo_name,
            request_id,
            session_id,
            revision,
            source.as_ref().map(|source| &source.locator_binding),
        )?)
    }

    pub(crate) fn is_exact_apply_replay(
        &self,
        repo_id: RepoId,
        request_id: Uuid,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    ) -> Result<bool, RemoteImportHostError> {
        Ok(RemoteImportService::open(&self.repo, repo_id)?
            .is_exact_apply_replay(&self.repo, request_id, session_id, revision)?)
    }

    pub(crate) fn discard(
        &self,
        repo_id: RepoId,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    ) -> Result<RemoteImportSessionView, RemoteImportHostError> {
        Ok(RemoteImportService::open(&self.repo, repo_id)?.discard(session_id, revision)?)
    }

    pub(crate) fn inspect_repair(
        &self,
        repo_id: RepoId,
    ) -> Result<RemoteImportRepairPlan, RemoteImportHostError> {
        Ok(RemoteImportService::inspect_repair(&self.repo, repo_id)?)
    }

    pub(crate) fn apply_repair(
        &self,
        repo_id: RepoId,
        expected_token: &str,
    ) -> Result<RemoteImportRepairPlan, RemoteImportHostError> {
        Ok(RemoteImportService::open(&self.repo, repo_id)?.apply_repair(expected_token)?)
    }

    fn resolve_source(
        &self,
        repo_name: &str,
        provider_hint: Option<RemoteProjectionProvider>,
    ) -> Result<ResolvedRemoteSource, RemoteImportHostError> {
        let raw_locator = self
            .repo
            .get_repo_url(None, repo_name)
            .map_err(|error| RemoteImportHostError::Locator(error.to_string()))?
            .ok_or_else(|| {
                RemoteImportHostError::Locator(
                    "remote projection locator is not configured".to_string(),
                )
            })?;
        let provider = match provider_hint {
            Some(provider) => provider,
            None => infer_provider(&raw_locator)?,
        };
        let request = SourceAcquisitionRequest::new(provider, raw_locator)
            .map_err(|error| RemoteImportHostError::Locator(error.to_string()))?;
        let locator = request.locator().to_string();
        let (s3_provider, profile_id) = if provider == RemoteProjectionProvider::S3 {
            let (provider, profile_id) =
                crate::remote_projection_transport::s3::source_provider_for_locator(
                    self.repo.ledger_dir(),
                    &locator,
                )
                .map_err(|error| RemoteImportHostError::Provider(error.to_string()))?;
            (Some(provider), profile_id)
        } else {
            (None, None)
        };
        let source_identity = canonical_binding_material(provider, None, profile_id.as_deref());
        let locator_identity =
            canonical_binding_material(provider, Some(&locator), profile_id.as_deref());
        Ok(ResolvedRemoteSource {
            provider,
            locator,
            source_binding: RemoteImportBinding::from_canonical_identity(
                "source",
                &source_identity,
            ),
            locator_binding: RemoteImportBinding::from_canonical_identity(
                "locator-profile",
                &locator_identity,
            ),
            s3_provider,
        })
    }

    fn acquire_apply(&self, repo_id: RepoId) -> Result<ApplyLease<'_>, RemoteImportHostError> {
        let mut applying = self
            .applying
            .lock()
            .map_err(|_| RemoteImportHostError::Coordination)?;
        if !applying.insert(repo_id) {
            return Err(RemoteImportHostError::ApplyBusy);
        }
        Ok(ApplyLease {
            applying: &self.applying,
            repo_id,
        })
    }

    fn acquire_provider(
        &self,
        repo_id: RepoId,
    ) -> Result<ProviderTaskLease<'_>, RemoteImportHostError> {
        self.providers.acquire(repo_id).map_err(Into::into)
    }
}

struct CaptureBridge<'a>(&'a mut deve_core::remote_import::RemoteImportCaptureSink);

impl RemoteSourceSink for CaptureBridge<'_> {
    type Error = deve_core::remote_import::RemoteImportError;

    fn capture(
        &mut self,
        path: &NormalizedRemotePath,
        body: &mut dyn Read,
    ) -> RemoteImportResult<()> {
        self.0.capture_file(path.as_str(), body)
    }
}

struct ApplyLease<'a> {
    applying: &'a Mutex<HashSet<RepoId>>,
    repo_id: RepoId,
}

impl Drop for ApplyLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut applying) = self.applying.lock() {
            applying.remove(&self.repo_id);
        }
    }
}
