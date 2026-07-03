//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-catalog-repair-contract

use crate::ledger::manager::types::{RepoInfo, RepoManager};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) fn ensure_local_repo_metadata_name_authorized(
    ledger_dir: &Path,
    stem: &str,
    info: &RepoInfo,
) -> Result<()> {
    if info.name == stem {
        return Ok(());
    }
    if crate::ledger::manager::projection_locator::locator_authorizes_repo_name(
        ledger_dir, info.uuid, &info.name,
    )? {
        return Ok(());
    }
    Err(anyhow!(
        "Local repo {} metadata name drifted to {}",
        stem,
        info.name
    ))
}

pub(super) fn validate_local_repo_info(
    stem: &str,
    _expected_name: &str,
    info: Option<RepoInfo>,
    seen: &mut HashMap<uuid::Uuid, String>,
    seen_urls: &mut HashMap<String, String>,
    seen_names: &mut HashMap<String, String>,
) -> Result<()> {
    let info = info.ok_or_else(|| {
        anyhow!(
            "Broken local repo {} while validating catalog: repository metadata missing",
            stem
        )
    })?;
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
    if let Some(owner) = seen_names.insert(info.name.clone(), stem.to_string())
        && owner != stem
    {
        return Err(anyhow!(
            "Broken local repo {} while validating catalog: duplicate local repository display name {} also used by {}",
            stem,
            info.name,
            owner
        ));
    }
    Ok(())
}

pub(super) struct WorkspaceRootRepairPlan {
    stem: String,
    repo_id: uuid::Uuid,
    current_name: String,
    old_root: PathBuf,
    new_root: PathBuf,
}

pub(super) fn prepare_workspace_root_repair(
    ledger_dir: &Path,
    stem: &str,
    repo_id: uuid::Uuid,
    previous_name: &str,
    current_name: &str,
) -> Result<Option<WorkspaceRootRepairPlan>> {
    if previous_name.trim().is_empty() {
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
    let previous_segment =
        crate::ledger::manager::projection_locator::repo_workspace_segment(previous_name, repo_id)?;
    let current_segment =
        crate::ledger::manager::projection_locator::repo_workspace_segment(current_name, repo_id)?;
    let previous_legacy_segment =
        crate::ledger::manager::projection_locator::safe_repo_path_segment(previous_name)?;
    let current_legacy_segment =
        crate::ledger::manager::projection_locator::safe_repo_path_segment(current_name)?;

    let new_root = projection_base_abs.join(&current_segment);
    let mut source_candidates = Vec::new();
    if previous_name != current_name {
        source_candidates.push(projection_base_abs.join(&previous_segment));
        source_candidates.push(projection_base_abs.join(&previous_legacy_segment));
    }
    source_candidates.push(projection_base_abs.join(&current_legacy_segment));
    source_candidates.sort();
    source_candidates.dedup();

    let new_exists = path_exists(&new_root, "current workspace root")?;
    let existing_sources = source_candidates
        .into_iter()
        .filter(|path| path != &new_root)
        .map(|path| {
            path_exists(&path, "previous workspace root").map(|exists| exists.then_some(path))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if new_exists && !existing_sources.is_empty() {
        return Err(anyhow!(
            "Broken local repo {} while repairing local catalog: current workspace root {:?} already exists",
            current_name,
            new_root
        ));
    }
    if new_exists || existing_sources.is_empty() {
        return Ok(None);
    }
    if existing_sources.len() > 1 {
        return Err(anyhow!(
            "Broken local repo {} while repairing local catalog: multiple candidate workspace roots exist: {:?}",
            current_name,
            existing_sources
        ));
    }
    let old_root = existing_sources.into_iter().next().expect("checked len");
    Ok(Some(WorkspaceRootRepairPlan {
        stem: stem.to_string(),
        repo_id,
        current_name: current_name.to_string(),
        old_root,
        new_root,
    }))
}

fn path_exists(path: &Path, label: &str) -> Result<bool> {
    path.try_exists()
        .with_context(|| format!("Failed to stat {label} while repairing local catalog: {path:?}"))
}

pub(super) fn preflight_workspace_root_repair(
    manager: &RepoManager,
    plan: &WorkspaceRootRepairPlan,
) -> Result<()> {
    let pending = manager.run_on_local_repo_stem(&plan.stem, |db| {
        crate::source_control::pending_fs::list_all(db)
    })?;
    if !pending.is_empty() {
        return Err(anyhow!(
            "Workspace root realign for {} refused: {} pending workspace change(s)",
            plan.current_name,
            pending.len()
        ));
    }
    let staged = manager.run_on_local_repo_stem(&plan.stem, |db| {
        crate::source_control::staging::list_staged_entries(db)
    })?;
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

        let diagnostic = crate::sync::diagnose_projection_local_repo_stem(
            manager,
            &plan.current_name,
            &plan.stem,
        )?;
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

        let drift = crate::sync::drift_detect::detect_repo_drift_at_workspace_root_stem(
            manager,
            &plan.stem,
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
    crate::utils::notegit::ensure_repo_identity_marker(
        &plan.new_root,
        plan.repo_id,
        &plan.current_name,
    )?;
    tracing::warn!(
        "Realigned local workspace root: {} -> {}",
        previous_name,
        plan.current_name
    );
    Ok(())
}
