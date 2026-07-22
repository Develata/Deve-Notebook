//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 03_storage/index#repo-runtime-layout
//!
//! Host-owned construction and exact revalidation of removal manifests.

use super::RepoLifecycleHostExecutor;
use crate::server::runtime::repo_lifecycle_job_runtime::model::{JobFuture, RepoLifecycleJobError};
use crate::server::runtime::repo_lifecycle_job_runtime::removal::{
    RepoRemovalFallbackSnapshot, RepoRemovalManifest, RepoRemovalPlanner, RepoRemovalPreparation,
    RepoRemovalPrepareIntent,
};
use deve_core::ledger::{RepoCatalogMembershipState, RepoManager};
use deve_core::models::RepoId;
use deve_core::protocol::{
    LocalRepoRemovalBlocker, LocalRepoRemovalDeletedCategory, LocalRepoRemovalPreservedCategory,
    LocalRepoRemovalPreview, LocalRepoRemovalWarning,
};
use deve_core::remote_import::{
    RemoteImportRepoRemovalAdmission, RemoteImportRepoRemovalBlocker,
    RemoteImportRepoRemovalRevalidation,
};
use deve_core::utils::fs::{HostPathIdentity, HostPathKind};
use deve_core::utils::notegit;
use sha2::{Digest, Sha256};
use std::io::Read;

const IDENTITY_MARKER_MAX_BYTES: u64 = 64 * 1024;

impl RepoRemovalPlanner for RepoLifecycleHostExecutor {
    fn prepare_removal(
        &self,
        intent: RepoRemovalPrepareIntent,
    ) -> JobFuture<Result<RepoRemovalPreparation, RepoLifecycleJobError>> {
        let repo = self.repo.clone();
        let watcher = self.watcher.clone();
        let sync_manager = self.sync_manager.clone();
        let remote_import = self.remote_import.clone();
        Box::pin(async move {
            build_preparation(
                &repo,
                &watcher,
                &sync_manager,
                &remote_import,
                intent.repo_id,
                intent.fallback_repo_id,
            )
        })
    }

    fn revalidate_removal(
        &self,
        manifest: RepoRemovalManifest,
    ) -> JobFuture<Result<(), RepoLifecycleJobError>> {
        let repo = self.repo.clone();
        let watcher = self.watcher.clone();
        let sync_manager = self.sync_manager.clone();
        let remote_import = self.remote_import.clone();
        Box::pin(
            async move { revalidate(&repo, &watcher, &sync_manager, &remote_import, &manifest) },
        )
    }
}

fn build_preparation(
    repo: &RepoManager,
    watcher: &crate::server::runtime::watcher_runtime::WatcherRuntimeView,
    sync_manager: &deve_core::sync::SyncManager,
    remote_import: &crate::remote_import_runtime::RemoteImportCoordinator,
    repo_id: RepoId,
    fallback_repo_id: Option<RepoId>,
) -> Result<RepoRemovalPreparation, RepoLifecycleJobError> {
    let mut preview = preview_base(fallback_repo_id.is_none());
    let catalog = match repo.repo_catalog_membership_record(repo_id) {
        Ok(Some(record)) if record.state() == RepoCatalogMembershipState::Normal => Some(record),
        _ => {
            push_unique(
                &mut preview.blockers,
                LocalRepoRemovalBlocker::RepositoryIdentityAmbiguous,
            );
            None
        }
    };
    let authority = match repo.snapshot_local_authority_for_removal(repo_id) {
        Ok(snapshot) => Some(snapshot),
        Err(
            deve_core::ledger::LocalAuthorityError::Busy(_)
            | deve_core::ledger::LocalAuthorityError::Quiescing(_),
        ) => {
            push_unique(
                &mut preview.blockers,
                LocalRepoRemovalBlocker::AuthorityBusy,
            );
            None
        }
        Err(_) => {
            push_unique(
                &mut preview.blockers,
                LocalRepoRemovalBlocker::RepositoryIdentityAmbiguous,
            );
            None
        }
    };
    let locator = match repo.validated_projection_locator_for_repo_id(repo_id) {
        Ok(locator) => Some(locator),
        Err(_) => {
            push_unique(
                &mut preview.blockers,
                LocalRepoRemovalBlocker::RepositoryIdentityAmbiguous,
            );
            None
        }
    };
    let workspace_identity = locator
        .as_ref()
        .and_then(|locator| workspace_identities(locator, repo_id).ok());
    if workspace_identity.is_none() {
        push_unique(
            &mut preview.blockers,
            LocalRepoRemovalBlocker::WorkspaceIdentityUnverified,
        );
    }
    let watcher_generation = match watcher.admit(repo_id) {
        Ok(token) => Some(token.generation()),
        Err(_) => {
            push_unique(
                &mut preview.blockers,
                LocalRepoRemovalBlocker::WorkspaceIngestionUnavailable,
            );
            None
        }
    };
    match projection_degraded(repo, sync_manager, repo_id) {
        Ok(true) => push_unique(
            &mut preview.blockers,
            LocalRepoRemovalBlocker::ProjectionFault,
        ),
        Ok(false) => {}
        Err(_) => push_unique(
            &mut preview.blockers,
            LocalRepoRemovalBlocker::RepairRequired,
        ),
    }
    let alias_revision = match repo.host_repo_alias_runtime().binding(repo_id) {
        Ok(binding) => Some(binding.alias_revision),
        Err(_) => {
            push_unique(
                &mut preview.blockers,
                LocalRepoRemovalBlocker::RepositoryIdentityAmbiguous,
            );
            None
        }
    };
    let remote_snapshot = match remote_import.repo_removal_admission(repo_id) {
        Ok(RemoteImportRepoRemovalAdmission::Admitted(snapshot)) => Some(snapshot),
        Ok(RemoteImportRepoRemovalAdmission::Blocked(blocked)) => {
            for blocker in blocked.blockers() {
                let blocker = match blocker {
                    RemoteImportRepoRemovalBlocker::ProjectionPending { .. } => {
                        LocalRepoRemovalBlocker::RemoteImportProjectionPending
                    }
                    RemoteImportRepoRemovalBlocker::ProjectionDegraded { .. } => {
                        LocalRepoRemovalBlocker::RemoteImportProjectionDegraded
                    }
                };
                push_unique(&mut preview.blockers, blocker);
            }
            None
        }
        Err(crate::remote_import_runtime::RemoteImportHostError::ApplyBusy) => {
            push_unique(
                &mut preview.blockers,
                LocalRepoRemovalBlocker::RemoteImportApplyInFlight,
            );
            None
        }
        Err(_) => {
            push_unique(
                &mut preview.blockers,
                LocalRepoRemovalBlocker::RepairRequired,
            );
            None
        }
    };
    if remote_snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.capture_cleanup_required())
    {
        push_unique(
            &mut preview.warnings,
            LocalRepoRemovalWarning::RemoteImportCaptureWillBeDiscarded,
        );
    }
    let fallback = fallback_repo_id.and_then(|fallback_repo_id| {
        match fallback_snapshot(repo, watcher, sync_manager, repo_id, fallback_repo_id) {
            Ok(snapshot) => Some(snapshot),
            Err(()) => {
                preview
                    .warnings
                    .push(LocalRepoRemovalWarning::SelectedFallbackUnavailable);
                None
            }
        }
    });
    let manifest = match (
        catalog,
        authority,
        locator,
        workspace_identity,
        alias_revision,
        watcher_generation,
        remote_snapshot,
    ) {
        (
            Some(catalog),
            Some(authority),
            Some(locator),
            Some((workspace_root, notegit_root, identity_marker, identity_marker_digest)),
            Some(alias_revision),
            Some(watcher_generation),
            Some(remote_import),
        ) => Some(RepoRemovalManifest {
            repo_id,
            catalog,
            authority,
            locator,
            workspace_root,
            notegit_root,
            identity_marker,
            identity_marker_digest,
            alias_revision,
            watcher_generation,
            remote_import,
            fallback,
        }),
        _ => None,
    };
    Ok(RepoRemovalPreparation { manifest, preview })
}

fn revalidate(
    repo: &RepoManager,
    watcher: &crate::server::runtime::watcher_runtime::WatcherRuntimeView,
    sync_manager: &deve_core::sync::SyncManager,
    remote_import: &crate::remote_import_runtime::RemoteImportCoordinator,
    manifest: &RepoRemovalManifest,
) -> Result<(), RepoLifecycleJobError> {
    let current_locator = repo
        .validated_projection_locator_for_repo_id(manifest.repo_id)
        .ok()
        .filter(|locator| locator == &manifest.locator);
    let current_workspace = current_locator
        .as_ref()
        .and_then(|locator| workspace_identities(locator, manifest.repo_id).ok());
    let exact = repo
        .repo_catalog_membership_record(manifest.repo_id)
        .ok()
        .flatten()
        .is_some_and(|record| record == manifest.catalog)
        && repo
            .snapshot_local_authority_for_removal(manifest.repo_id)
            .ok()
            .is_some_and(|snapshot| snapshot == manifest.authority)
        && manifest.authority.revalidate().unwrap_or(false)
        && current_workspace.is_some_and(
            |(workspace_root, notegit_root, identity_marker, identity_marker_digest)| {
                workspace_root == manifest.workspace_root
                    && notegit_root == manifest.notegit_root
                    && identity_marker == manifest.identity_marker
                    && identity_marker_digest == manifest.identity_marker_digest
            },
        )
        && repo
            .host_repo_alias_runtime()
            .binding(manifest.repo_id)
            .ok()
            .is_some_and(|binding| binding.alias_revision == manifest.alias_revision)
        && watcher
            .admit(manifest.repo_id)
            .ok()
            .is_some_and(|token| token.generation() == manifest.watcher_generation)
        && !projection_degraded(repo, sync_manager, manifest.repo_id).unwrap_or(true)
        && matches!(
            remote_import.revalidate_repo_removal(manifest.repo_id, &manifest.remote_import),
            Ok(RemoteImportRepoRemovalRevalidation::Exact)
        )
        && fallback_revalidates(repo, watcher, sync_manager, manifest);
    if exact {
        Ok(())
    } else {
        Err(RepoLifecycleJobError::ConfirmationStale)
    }
}

fn workspace_identities(
    locator: &deve_core::ledger::ProjectionLocatorRecord,
    repo_id: RepoId,
) -> anyhow::Result<(HostPathIdentity, HostPathIdentity, HostPathIdentity, String)> {
    let workspace_root =
        std::fs::canonicalize(locator.projection_base_abs.join(&locator.workspace_segment))?;
    let marker_path = notegit::repo_identity_path(&workspace_root);
    let marker_file =
        deve_core::utils::fs::open_regular_file_read(&marker_path, "repo removal identity marker")?;
    let marker_len = marker_file.metadata()?.len();
    if marker_len > IDENTITY_MARKER_MAX_BYTES {
        anyhow::bail!("repo identity marker exceeds removal admission budget");
    }
    let mut marker_bytes = Vec::with_capacity(marker_len as usize);
    (&marker_file)
        .take(IDENTITY_MARKER_MAX_BYTES + 1)
        .read_to_end(&mut marker_bytes)?;
    if marker_bytes.len() as u64 > IDENTITY_MARKER_MAX_BYTES {
        anyhow::bail!("repo identity marker exceeds removal admission budget");
    }
    notegit::validate_repo_identity_marker_content(&marker_bytes, &workspace_root, repo_id)?;
    deve_core::utils::fs::ensure_open_file_matches_path(
        &marker_file,
        &marker_path,
        "repo removal identity marker",
    )?;
    let marker_identity = HostPathIdentity::capture(&marker_path, HostPathKind::RegularFile)?;
    Ok((
        HostPathIdentity::capture(&workspace_root, HostPathKind::Directory)?,
        HostPathIdentity::capture(&notegit::repo_dir(&workspace_root), HostPathKind::Directory)?,
        marker_identity,
        format!("{:x}", Sha256::digest(&marker_bytes)),
    ))
}

fn fallback_snapshot(
    repo: &RepoManager,
    watcher: &crate::server::runtime::watcher_runtime::WatcherRuntimeView,
    sync_manager: &deve_core::sync::SyncManager,
    removed_repo_id: RepoId,
    fallback_repo_id: RepoId,
) -> Result<RepoRemovalFallbackSnapshot, ()> {
    if removed_repo_id == fallback_repo_id
        || projection_degraded(repo, sync_manager, fallback_repo_id).unwrap_or(true)
    {
        return Err(());
    }
    let catalog = repo
        .repo_catalog_membership_record(fallback_repo_id)
        .map_err(|_| ())?
        .filter(|record| record.state() == RepoCatalogMembershipState::Normal)
        .ok_or(())?;
    let authority = repo
        .snapshot_local_authority_for_removal(fallback_repo_id)
        .map_err(|_| ())?;
    let mount = watcher.admit(fallback_repo_id).map_err(|_| ())?;
    Ok(RepoRemovalFallbackSnapshot {
        repo_id: fallback_repo_id,
        membership_revision: catalog.membership_revision(),
        authority_generation: authority.generation(),
        watcher_generation: mount.generation(),
    })
}

fn fallback_revalidates(
    repo: &RepoManager,
    watcher: &crate::server::runtime::watcher_runtime::WatcherRuntimeView,
    sync_manager: &deve_core::sync::SyncManager,
    manifest: &RepoRemovalManifest,
) -> bool {
    match &manifest.fallback {
        None => true,
        Some(expected) => fallback_snapshot(
            repo,
            watcher,
            sync_manager,
            manifest.repo_id,
            expected.repo_id,
        )
        .is_ok_and(|actual| actual == *expected),
    }
}

fn projection_degraded(
    repo: &RepoManager,
    sync_manager: &deve_core::sync::SyncManager,
    repo_id: RepoId,
) -> anyhow::Result<bool> {
    let execution_name = repo
        .find_local_repo_name_by_id(repo_id)?
        .ok_or_else(|| anyhow::anyhow!("local repo is absent"))?;
    Ok(sync_manager
        .degraded_local_repo_names_for_execution()?
        .iter()
        .any(|name| name == &execution_name))
}

fn preview_base(no_fallback: bool) -> LocalRepoRemovalPreview {
    let mut warnings = vec![LocalRepoRemovalWarning::LedgerHistoryHasNoSupportedRestore];
    if no_fallback {
        warnings.push(LocalRepoRemovalWarning::NoFallbackSelected);
    }
    LocalRepoRemovalPreview {
        deleted: vec![
            LocalRepoRemovalDeletedCategory::LocalLedgerAuthority,
            LocalRepoRemovalDeletedCategory::DeveRuntimeMetadata,
            LocalRepoRemovalDeletedCategory::ProjectionLocator,
            LocalRepoRemovalDeletedCategory::HostAlias,
            LocalRepoRemovalDeletedCategory::RemoteImportCaptures,
            LocalRepoRemovalDeletedCategory::CatalogMembership,
        ],
        preserved: vec![
            LocalRepoRemovalPreservedCategory::WorkspaceContent,
            LocalRepoRemovalPreservedCategory::GitMetadata,
            LocalRepoRemovalPreservedCategory::RemoteShadows,
            LocalRepoRemovalPreservedCategory::HostIdentityAndConfiguration,
            LocalRepoRemovalPreservedCategory::OperatorRecoveryInputs,
            LocalRepoRemovalPreservedCategory::AuthorityLockIdentity,
        ],
        warnings,
        blockers: Vec::new(),
    }
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}
