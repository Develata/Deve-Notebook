//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 07_network#repo-control-wire-contract
//!
//! Host adapters that connect the transport-neutral lifecycle owner to the
//! repository coordinator and bounded publication surfaces.

use super::model::{
    AdmittedRepoLifecycleJob, JobFuture, RepoLifecycleJobCompletion, RepoLifecycleJobExecutor,
    RepoLifecyclePublicationSink, RepoLifecycleSettledPublication,
};
use crate::server::runtime::repo_lifecycle_runtime::{
    CreateRepoIntent, RepoLifecycleCoordinator, RepoLifecycleError,
};
use crate::server::runtime::repo_session_runtime::FinalRepoListProjection;
use crate::server::runtime::watcher_runtime::WatcherRuntimeView;
use deve_core::ledger::{RepoCatalogMembershipState, RepoManager};
use deve_core::protocol::{RepoListEntry, ServerMessage};
use std::sync::Arc;
use tokio::sync::broadcast;

pub(crate) struct RepoLifecycleHostExecutor {
    coordinator: Arc<RepoLifecycleCoordinator>,
    repo: Arc<RepoManager>,
    watcher: WatcherRuntimeView,
}

impl RepoLifecycleHostExecutor {
    pub(crate) fn new(
        coordinator: Arc<RepoLifecycleCoordinator>,
        repo: Arc<RepoManager>,
        watcher: WatcherRuntimeView,
    ) -> Self {
        Self {
            coordinator,
            repo,
            watcher,
        }
    }

    fn classify_failure(
        &self,
        job: &AdmittedRepoLifecycleJob,
        error: RepoLifecycleError,
    ) -> RepoLifecycleJobCompletion {
        let detail = error.to_string();
        if matches!(error, RepoLifecycleError::RepairRequired { .. }) {
            return RepoLifecycleJobCompletion::repair_required(detail);
        }
        match self.repo.repo_catalog_membership_record(job.target_repo_id) {
            Ok(Some(record))
                if record.confirms_created(job.request_id)
                    && job.intent.create_parts().is_some() =>
            {
                RepoLifecycleJobCompletion::committed_partial_with_publication(
                    detail,
                    RepoLifecycleSettledPublication::Created {
                        repo_id: job.target_repo_id,
                        mounted: matches!(
                            self.watcher.repo_readiness(job.target_repo_id),
                            deve_core::protocol::RepoReadiness::Mounted
                        ),
                    },
                )
            }
            Ok(Some(record))
                if record.confirms_removed(job.request_id)
                    && job.intent.remove_repo_id().is_some() =>
            {
                RepoLifecycleJobCompletion::committed_partial_with_publication(
                    detail,
                    RepoLifecycleSettledPublication::Removed {
                        repo_id: job.target_repo_id,
                        fallback_repo_id: None,
                    },
                )
            }
            Ok(_) => RepoLifecycleJobCompletion::not_committed(detail),
            Err(classify) => RepoLifecycleJobCompletion::repair_required(format!(
                "{detail}; catalog truth classification failed: {classify}"
            )),
        }
    }

    fn recover_from_truth(&self, job: &AdmittedRepoLifecycleJob) -> RepoLifecycleJobCompletion {
        let record = match self.repo.repo_catalog_membership_record(job.target_repo_id) {
            Ok(record) => record,
            Err(error) => {
                return RepoLifecycleJobCompletion::repair_required(format!(
                    "cannot read catalog truth during lifecycle recovery: {error}"
                ));
            }
        };
        match (
            job.intent.create_parts(),
            job.intent.remove_repo_id(),
            record,
        ) {
            (Some(_), None, Some(record)) if record.confirms_created(job.request_id) => {
                match self.repo.list_cataloged_local_repo_summaries() {
                    Ok(summaries)
                        if summaries
                            .iter()
                            .any(|summary| summary.repo_id == job.target_repo_id) =>
                    {
                        RepoLifecycleJobCompletion::committed_partial_with_publication(
                            "create was committed before restart; settlement was reconstructed",
                            RepoLifecycleSettledPublication::Created {
                                repo_id: job.target_repo_id,
                                mounted: matches!(
                                    self.watcher.repo_readiness(job.target_repo_id),
                                    deve_core::protocol::RepoReadiness::Mounted
                                ),
                            },
                        )
                    }
                    Ok(_) => RepoLifecycleJobCompletion::repair_required(
                        "normal create catalog record has no exact local repo projection",
                    ),
                    Err(error) => RepoLifecycleJobCompletion::repair_required(format!(
                        "create recovery truth is inconsistent: {error}"
                    )),
                }
            }
            (None, Some(repo_id), Some(record))
                if repo_id == job.target_repo_id && record.confirms_removed(job.request_id) =>
            {
                RepoLifecycleJobCompletion::committed_partial_with_publication(
                    "remove was committed before restart; settlement was reconstructed",
                    RepoLifecycleSettledPublication::Removed {
                        repo_id,
                        fallback_repo_id: None,
                    },
                )
            }
            (Some(_), None, None) => RepoLifecycleJobCompletion::repair_required(
                "interrupted create has no committed record; prepared artifacts require exact repair",
            ),
            (None, Some(_), Some(record))
                if record.state() == RepoCatalogMembershipState::Normal =>
            {
                RepoLifecycleJobCompletion::not_committed(
                    "interrupted remove left the catalog membership normal",
                )
            }
            _ => RepoLifecycleJobCompletion::repair_required(
                "lifecycle receipt and catalog truth do not identify one recovery state",
            ),
        }
    }
}

impl RepoLifecycleJobExecutor for RepoLifecycleHostExecutor {
    fn execute(&self, job: AdmittedRepoLifecycleJob) -> JobFuture<RepoLifecycleJobCompletion> {
        let coordinator = self.coordinator.clone();
        let repo = self.repo.clone();
        let watcher = self.watcher.clone();
        Box::pin(async move {
            let executor = Self {
                coordinator: coordinator.clone(),
                repo,
                watcher,
            };
            let result = if let Some((initial_alias, projection_base, projection_base_repo_id)) =
                job.intent.create_parts()
            {
                if let Err(error) = coordinator
                    .revalidate_create_projection_base(projection_base_repo_id, projection_base)
                {
                    return executor.classify_failure(&job, error);
                }
                coordinator
                    .create(CreateRepoIntent {
                        repo_id: job.target_repo_id,
                        initial_alias: initial_alias.to_string(),
                        projection_base: projection_base.to_path_buf(),
                        lifecycle_request_id: job.request_id,
                    })
                    .await
                    .map(|outcome| {
                        let publication = RepoLifecycleSettledPublication::Created {
                            repo_id: job.target_repo_id,
                            mounted: outcome.mount.is_mounted(),
                        };
                        if outcome.mount.is_mounted() {
                            RepoLifecycleJobCompletion::succeeded(publication)
                        } else {
                            RepoLifecycleJobCompletion::committed_partial_with_publication(
                                "repository committed but workspace ingestion is unavailable",
                                publication,
                            )
                        }
                    })
            } else if let Some(repo_id) = job.intent.remove_repo_id() {
                coordinator
                    .remove(repo_id, job.request_id)
                    .await
                    .map(|outcome| {
                        let publication = RepoLifecycleSettledPublication::Removed {
                            repo_id,
                            fallback_repo_id: outcome
                                .fallback
                                .as_ref()
                                .map(|fallback| fallback.summary().repo_id),
                        };
                        if outcome.repair_required {
                            RepoLifecycleJobCompletion::committed_partial_with_publication(
                                "repository removed with cleanup debt",
                                publication,
                            )
                        } else {
                            RepoLifecycleJobCompletion::succeeded(publication)
                        }
                    })
            } else {
                return RepoLifecycleJobCompletion::repair_required(
                    "lifecycle intent has no executable operation",
                );
            };
            result.unwrap_or_else(|error| executor.classify_failure(&job, error))
        })
    }

    fn recover(&self, job: AdmittedRepoLifecycleJob) -> JobFuture<RepoLifecycleJobCompletion> {
        let repo = self.repo.clone();
        let watcher = self.watcher.clone();
        let coordinator = self.coordinator.clone();
        Box::pin(async move {
            Self {
                coordinator,
                repo,
                watcher,
            }
            .recover_from_truth(&job)
        })
    }

    fn retain_create_receipt(&self, repo_id: deve_core::models::RepoId) -> bool {
        self.repo
            .repo_catalog_membership_record(repo_id)
            .ok()
            .flatten()
            .is_some_and(|record| record.state() == RepoCatalogMembershipState::Normal)
    }
}

pub(crate) struct RepoLifecycleHostPublicationSink {
    repo: Arc<RepoManager>,
    watcher: WatcherRuntimeView,
    sessions: Arc<crate::server::runtime::repo_session_runtime::RepoSessionRuntime>,
    tx: broadcast::Sender<ServerMessage>,
}

impl RepoLifecycleHostPublicationSink {
    pub(crate) fn new(
        repo: Arc<RepoManager>,
        watcher: WatcherRuntimeView,
        sessions: Arc<crate::server::runtime::repo_session_runtime::RepoSessionRuntime>,
        tx: broadcast::Sender<ServerMessage>,
    ) -> Self {
        Self {
            repo,
            watcher,
            sessions,
            tx,
        }
    }

    fn final_projection(&self) -> Result<FinalRepoListProjection, String> {
        let entries = self
            .repo
            .list_cataloged_local_repo_summaries()
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|summary| {
                let alias = self
                    .repo
                    .host_repo_alias_runtime()
                    .binding(summary.repo_id)
                    .map_err(|error| error.to_string())?;
                Ok(RepoListEntry {
                    repo_id: summary.repo_id,
                    display_alias: alias.alias,
                    alias_revision: alias.alias_revision,
                    readiness: self.watcher.repo_readiness(summary.repo_id),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(FinalRepoListProjection { entries })
    }
}

impl RepoLifecyclePublicationSink for RepoLifecycleHostPublicationSink {
    fn publish(
        &self,
        request_id: uuid::Uuid,
        publication: RepoLifecycleSettledPublication,
    ) -> JobFuture<Result<(), String>> {
        let projection = self.final_projection();
        let sessions = self.sessions.clone();
        let tx = self.tx.clone();
        Box::pin(async move {
            let projection = projection?;
            let initiator_id = sessions
                .publish_lifecycle_settlement(request_id, publication.clone(), projection.clone())
                .map_err(|error| error.to_string())?;
            if let RepoLifecycleSettledPublication::Removed { repo_id, .. } = publication {
                sessions
                    .invalidate_removed_repo_observers(repo_id, initiator_id, projection.clone())
                    .map_err(|error| error.to_string())?;
            }
            let _ = tx.send(ServerMessage::RepoList {
                request_id: None,
                branch: None,
                scope_nonce: None,
                repo_entries: projection.entries,
            });
            Ok(())
        })
    }
}
