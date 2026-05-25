//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-catalog-repair-contract

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) fn validate_local_repo_info(
    stem: &str,
    expected_name: &str,
    info: Option<RepoInfo>,
    seen: &mut HashMap<uuid::Uuid, String>,
    seen_urls: &mut HashMap<String, String>,
) -> Result<()> {
    let info = info.ok_or_else(|| {
        anyhow!(
            "Broken local repo {} while validating catalog: repository metadata missing",
            stem
        )
    })?;
    if info.name != expected_name {
        return Err(anyhow!(
            "Broken local repo {} while validating catalog: metadata name drifted to {}",
            stem,
            info.name
        ));
    }
    if let Some(owner) = seen.insert(info.uuid, stem.to_string())
        && owner != stem
    {
        return Err(anyhow!(
            "Broken local repo {} while validating catalog: duplicate local repository UUID {} also used by {}",
            stem,
            info.uuid,
            owner
        ));
    }
    let url = info.url.ok_or_else(|| {
        anyhow!(
            "Broken local repo {} while validating catalog: repository URL missing",
            stem
        )
    })?;
    if let Some(owner) = seen_urls.insert(url.clone(), stem.to_string())
        && owner != stem
    {
        return Err(anyhow!(
            "Broken local repo {} while validating catalog: duplicate local repository URL {} also used by {}",
            stem,
            url,
            owner
        ));
    }
    Ok(())
}

pub(super) struct WorkspaceRootRepairPlan {
    repo_id: uuid::Uuid,
    current_name: String,
    old_root: PathBuf,
    new_root: PathBuf,
}

pub(super) fn prepare_workspace_root_repair(
    ledger_dir: &Path,
    repo_id: uuid::Uuid,
    previous_name: &str,
    current_name: &str,
) -> Result<Option<WorkspaceRootRepairPlan>> {
    if previous_name == current_name || previous_name.trim().is_empty() {
        return Ok(None);
    }
    let locator_path =
        crate::ledger::manager::projection_locator::projection_locator_path_for(ledger_dir);
    let locator =
        crate::ledger::manager::projection_locator::read_projection_locator_file(&locator_path)?
            .locators
            .into_iter()
            .find(|record| record.repo_id == repo_id);
    let Some(locator) = locator else {
        return Ok(None);
    };
    let previous_name =
        crate::ledger::manager::projection_locator::safe_repo_path_segment(previous_name)?;
    let current_name =
        crate::ledger::manager::projection_locator::safe_repo_path_segment(current_name)?;
    if !locator.projection_base_abs.is_absolute() {
        return Err(anyhow!(
            "Broken local repo {} while repairing local catalog: Projection Locator base must be absolute",
            current_name
        ));
    }
    let projection_base_abs = std::fs::canonicalize(&locator.projection_base_abs).with_context(|| {
        format!(
            "Failed to canonicalize Projection Locator base while repairing local catalog: {:?}",
            locator.projection_base_abs
        )
    })?;
    let old_root = projection_base_abs.join(&previous_name);
    let new_root = projection_base_abs.join(&current_name);
    let old_exists = old_root.try_exists().with_context(|| {
        format!(
            "Failed to stat previous workspace root while repairing local catalog: {old_root:?}"
        )
    })?;
    let new_exists = new_root.try_exists().with_context(|| {
        format!("Failed to stat current workspace root while repairing local catalog: {new_root:?}")
    })?;
    if old_exists && new_exists {
        return Err(anyhow!(
            "Broken local repo {} while repairing local catalog: current workspace root {:?} already exists",
            current_name,
            new_root
        ));
    }
    if !old_exists {
        return Ok(None);
    }
    Ok(Some(WorkspaceRootRepairPlan {
        repo_id,
        current_name,
        old_root,
        new_root,
    }))
}

pub(super) fn preflight_workspace_root_repair(
    manager: &RepoManager,
    plan: &WorkspaceRootRepairPlan,
) -> Result<()> {
    let pending = manager.list_pending_fs_in_local_repo(&plan.current_name)?;
    if !pending.is_empty() {
        return Err(anyhow!(
            "Workspace root realign for {} refused: {} pending workspace change(s)",
            plan.current_name,
            pending.len()
        ));
    }
    let staged = manager.list_staged_in_local_repo(&plan.current_name)?;
    if !staged.is_empty() {
        return Err(anyhow!(
            "Workspace root realign for {} refused: {} staged source-control change(s)",
            plan.current_name,
            staged.len()
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if crate::sync::watcher::is_repo_watcher_running(plan.repo_id)
            .map_err(|err| anyhow!("Failed to inspect watcher state: {err}"))?
        {
            return Err(anyhow!(
                "Workspace root realign for {} refused: active watcher is running",
                plan.current_name
            ));
        }

        let diagnostic = crate::sync::diagnose_projection_local_repo(manager, &plan.current_name)?;
        if diagnostic.status != crate::sync::ProjectionDiagnosticStatus::Healthy {
            return Err(anyhow!(
                "Workspace root realign for {} refused: projection fault {}",
                plan.current_name,
                diagnostic
                    .issue
                    .as_ref()
                    .map(|issue| issue.detail.as_str())
                    .unwrap_or("unknown")
            ));
        }

        let drift = crate::sync::drift_detect::detect_repo_drift_at_workspace_root(
            manager,
            &plan.current_name,
            &plan.old_root,
        )?;
        if drift.is_fault() {
            return Err(anyhow!(
                "Workspace root realign for {} refused: dirty workspace has {} unexplained drift(s)",
                plan.current_name,
                drift.unexplained.len()
            ));
        }
    }

    Ok(())
}

pub(super) fn repair_workspace_root(plan: WorkspaceRootRepairPlan) -> Result<()> {
    let previous_name = plan
        .old_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unknown>")
        .to_string();
    std::fs::rename(&plan.old_root, &plan.new_root).with_context(|| {
        format!(
            "Failed to realign local workspace root from {:?} to {:?}",
            plan.old_root, plan.new_root
        )
    })?;
    tracing::warn!(
        "Realigned local workspace root: {} -> {}",
        previous_name,
        plan.current_name
    );
    Ok(())
}
