//! Sealed mutation publication vocabulary and wire ordering.

use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::{
    ConfirmedOp, DocumentRecoveryScope, ProjectionRecoveryCause, ProjectionRecoveryPlan,
    ProjectionRecoveryRequired, ServerMessage,
};
use tokio::sync::broadcast;

#[derive(Debug)]
pub(crate) enum MutationPublication {
    ConfirmedEdit {
        repo_id: RepoId,
        branch: Option<PeerId>,
        scope_nonce: Option<u64>,
        doc_id: DocId,
        entry: ConfirmedOp,
        recovery: Option<ProjectionRecoveryRequired>,
    },
    ProjectionRecovery(ProjectionRecoveryRequired),
    SourceControlCommit {
        repo_id: RepoId,
        branch: Option<PeerId>,
        scope_nonce: Option<u64>,
        commit_id: String,
        timestamp: i64,
        recovery: ProjectionRecoveryRequired,
    },
    MergeComplete {
        repo_id: RepoId,
        branch: Option<PeerId>,
        scope_nonce: Option<u64>,
        merged_count: u32,
        recovery: ProjectionRecoveryRequired,
    },
}

impl MutationPublication {
    pub(crate) fn projection_recovery(
        repo_id: RepoId,
        cause: ProjectionRecoveryCause,
        documents: DocumentRecoveryScope,
        refresh_doc_list: bool,
        refresh_source_control: bool,
        refresh_external_changes: bool,
    ) -> Self {
        Self::ProjectionRecovery(ProjectionRecoveryRequired {
            repo_id,
            branch: None,
            scope_nonce: None,
            cause,
            plan: ProjectionRecoveryPlan {
                documents,
                refresh_doc_list,
                refresh_source_control,
                refresh_external_changes,
            },
        })
    }

    pub(crate) fn document_recovery(repo_id: RepoId, documents: DocumentRecoveryScope) -> Self {
        Self::projection_recovery(
            repo_id,
            ProjectionRecoveryCause::DocumentMutation,
            documents,
            true,
            true,
            false,
        )
    }

    pub(crate) fn external_apply_recovery(repo_id: RepoId, affected_docs: Vec<DocId>) -> Self {
        Self::projection_recovery(
            repo_id,
            ProjectionRecoveryCause::ExternalApply,
            DocumentRecoveryScope::Exact(affected_docs),
            true,
            true,
            true,
        )
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn plugin_recovery(repo_id: RepoId, doc_id: DocId) -> Self {
        Self::projection_recovery(
            repo_id,
            ProjectionRecoveryCause::PluginMutation,
            DocumentRecoveryScope::Exact(vec![doc_id]),
            true,
            true,
            false,
        )
    }

    pub(crate) fn merge_recovery(repo_id: RepoId, doc_id: DocId) -> ProjectionRecoveryRequired {
        recovery_from(Self::projection_recovery(
            repo_id,
            ProjectionRecoveryCause::Merge,
            DocumentRecoveryScope::Exact(vec![doc_id]),
            true,
            true,
            false,
        ))
    }

    pub(crate) fn source_control_recovery(repo_id: RepoId) -> ProjectionRecoveryRequired {
        recovery_from(Self::projection_recovery(
            repo_id,
            ProjectionRecoveryCause::SourceControlCommit,
            DocumentRecoveryScope::None,
            false,
            true,
            false,
        ))
    }

    pub(super) fn enqueue(&self, tx: &broadcast::Sender<ServerMessage>) {
        match self {
            Self::ConfirmedEdit {
                repo_id,
                branch,
                scope_nonce,
                doc_id,
                entry,
                recovery,
            } => {
                let _ = tx.send(ServerMessage::NewOp {
                    repo_id: *repo_id,
                    branch: branch.clone(),
                    scope_nonce: *scope_nonce,
                    doc_id: *doc_id,
                    entry: entry.clone(),
                });
                if let Some(recovery) = recovery {
                    let _ = tx.send(ServerMessage::ProjectionRecoveryRequired(recovery.clone()));
                }
            }
            Self::ProjectionRecovery(recovery) => {
                let _ = tx.send(ServerMessage::ProjectionRecoveryRequired(recovery.clone()));
            }
            Self::SourceControlCommit {
                repo_id,
                branch,
                scope_nonce,
                commit_id,
                timestamp,
                recovery,
            } => {
                let _ = tx.send(ServerMessage::CommitAck {
                    repo_id: Some(*repo_id),
                    branch: branch.clone(),
                    scope_nonce: *scope_nonce,
                    commit_id: commit_id.clone(),
                    timestamp: *timestamp,
                });
                let _ = tx.send(ServerMessage::ProjectionRecoveryRequired(recovery.clone()));
            }
            Self::MergeComplete {
                repo_id,
                branch,
                scope_nonce,
                merged_count,
                recovery,
            } => {
                let _ = tx.send(ServerMessage::MergeComplete {
                    repo_id: Some(*repo_id),
                    branch: branch.clone(),
                    scope_nonce: *scope_nonce,
                    merged_count: *merged_count,
                });
                let _ = tx.send(ServerMessage::ProjectionRecoveryRequired(recovery.clone()));
            }
        }
    }
}

fn recovery_from(publication: MutationPublication) -> ProjectionRecoveryRequired {
    match publication {
        MutationPublication::ProjectionRecovery(recovery) => recovery,
        _ => unreachable!("projection recovery constructor is stable"),
    }
}
