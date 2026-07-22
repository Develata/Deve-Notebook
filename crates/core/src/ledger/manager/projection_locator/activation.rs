//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 03_storage/projection#projection-locator-contract
//!
//! Bounded locator, workspace-root, and marker observation held across the
//! composed repo-creation activation cut.

use super::{ProjectionLocatorMapGuard, ProjectionLocatorRecord};
use crate::models::RepoId;
use crate::utils::fs::{
    HostPathIdentity, HostPathKind, ensure_open_file_matches_identity, open_regular_file_read,
};
use crate::utils::notegit;
use anyhow::{Context, Result, anyhow};
use std::io::Read;

const MARKER_REVALIDATION_LIMIT: u64 = 64 * 1024;

pub(crate) struct ProjectionLocatorActivationGuard {
    _map_guard: ProjectionLocatorMapGuard,
    locator: ProjectionLocatorRecord,
    store: HostPathIdentity,
    workspace_root: HostPathIdentity,
    marker: HostPathIdentity,
}

impl ProjectionLocatorActivationGuard {
    pub(crate) fn acquire(ledger_dir: &std::path::Path, repo_id: RepoId) -> Result<Self> {
        let map_guard = ProjectionLocatorMapGuard::acquire(ledger_dir)?;
        let locator_path = super::projection_locator_path_for(ledger_dir);
        let store = HostPathIdentity::capture(&locator_path, HostPathKind::RegularFile)
            .context("Projection Locator store identity is unavailable")?;
        let file = super::read_projection_locator_file(&locator_path)?;
        let mut locator = file
            .locators
            .into_iter()
            .find(|record| record.repo_id == repo_id)
            .ok_or_else(|| anyhow!("Projection Locator is missing for {repo_id}"))?;
        locator.projection_base_abs = std::fs::canonicalize(&locator.projection_base_abs)
            .context("Projection Locator base is unavailable")?;
        let workspace_path =
            std::fs::canonicalize(locator.projection_base_abs.join(&locator.workspace_segment))
                .context("Projection workspace root is unavailable")?;
        notegit::validate_repo_identity_marker(&workspace_path, repo_id)?;
        let workspace_root = HostPathIdentity::capture(&workspace_path, HostPathKind::Directory)?;
        let marker_path = notegit::repo_identity_path(&workspace_path);
        let marker = HostPathIdentity::capture(&marker_path, HostPathKind::RegularFile)?;
        validate_marker(&marker, repo_id, &workspace_path)?;
        Ok(Self {
            _map_guard: map_guard,
            locator,
            store,
            workspace_root,
            marker,
        })
    }

    pub(crate) fn revalidate(&self) -> Result<()> {
        if !self.store.revalidate()?
            || !self.workspace_root.revalidate()?
            || !self.marker.revalidate()?
        {
            return Err(anyhow!("prepared repository physical identity changed"));
        }
        validate_marker(
            &self.marker,
            self.locator.repo_id,
            self.workspace_root.path(),
        )
    }

    pub(crate) fn locator(&self) -> &ProjectionLocatorRecord {
        &self.locator
    }

    pub(crate) fn store(&self) -> &HostPathIdentity {
        &self.store
    }

    pub(crate) fn workspace_root(&self) -> &HostPathIdentity {
        &self.workspace_root
    }

    pub(crate) fn marker(&self) -> &HostPathIdentity {
        &self.marker
    }
}

fn validate_marker(
    marker: &HostPathIdentity,
    repo_id: RepoId,
    workspace_root: &std::path::Path,
) -> Result<()> {
    let mut file = open_regular_file_read(marker.path(), "repo identity marker")?;
    ensure_open_file_matches_identity(&file, marker, "repo identity marker")?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MARKER_REVALIDATION_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MARKER_REVALIDATION_LIMIT {
        return Err(anyhow!("repo identity marker exceeds validation budget"));
    }
    notegit::validate_repo_identity_marker_content(&bytes, workspace_root, repo_id)
}
