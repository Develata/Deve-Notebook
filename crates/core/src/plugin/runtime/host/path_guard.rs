//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
use crate::plugin::manifest::Capability;
use crate::utils::path::path_to_forward_slash;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[cfg(unix)]
pub(super) fn project_relative_path(cwd: &Path, path: &Path) -> Result<Option<String>, String> {
    let cwd = std::fs::canonicalize(cwd)
        .map_err(|e| format!("Failed to canonicalize project root {:?}: {}", cwd, e))?;
    let path = canonicalize_target(cwd.as_path(), path)?;
    Ok(path.strip_prefix(&cwd).ok().map(path_to_forward_slash))
}

pub(super) fn is_ledger_managed_write_target(path: &Path) -> Result<bool, String> {
    let cwd = canonical_project_root()?;
    let target = canonicalize_target(&cwd, path)?;
    if project_relative_from_canonical(&cwd, &target)
        .is_some_and(|rel| is_project_ledger_relative_path(&rel))
    {
        return Ok(true);
    }
    if let Ok(manager) = super::repo_manager()
        && is_ledger_managed_write_target_for(manager.as_ref(), path)?
    {
        return Ok(true);
    }
    Ok(false)
}

pub(super) fn resolve_capability_read_target(
    caps: &Capability,
    path: &Path,
) -> Result<Option<PathBuf>, String> {
    if !caps.check_read(path) {
        return Ok(None);
    }
    resolve_canonical_capability_target(&caps.allow_fs_read, path)
}

pub(super) fn resolve_capability_write_target(
    caps: &Capability,
    path: &Path,
) -> Result<Option<PathBuf>, String> {
    if !caps.check_write(path) {
        return Ok(None);
    }
    resolve_canonical_capability_target(&caps.allow_fs_write, path)
}

pub(super) fn managed_note_target_parts(path: &Path) -> Result<Option<(String, String)>, String> {
    if let Ok(manager) = super::repo_manager() {
        return managed_note_target_parts_for(manager.as_ref(), path);
    }
    Ok(None)
}

pub(super) fn is_ledger_managed_write_target_for(
    manager: &crate::ledger::RepoManager,
    path: &Path,
) -> Result<bool, String> {
    let cwd = canonical_project_root()?;
    let target = canonicalize_target(&cwd, path)?;
    if target.starts_with(&canonicalize_target(&cwd, &manager.ledger_dir)?) {
        return Ok(true);
    }
    let Some((_repo_name, repo_path)) =
        repo_workspace_relative_for_canonical_target(manager, &cwd, &target)?
    else {
        return Ok(false);
    };
    Ok(is_projection_workspace_managed_relative_path(&repo_path))
}

fn managed_note_target_parts_for(
    manager: &crate::ledger::RepoManager,
    path: &Path,
) -> Result<Option<(String, String)>, String> {
    let cwd = canonical_project_root()?;
    let target = canonicalize_target(&cwd, path)?;
    let Some((repo_name, repo_path)) =
        repo_workspace_relative_for_canonical_target(manager, &cwd, &target)?
    else {
        return Ok(None);
    };
    if is_projection_workspace_note_path(&repo_path) {
        Ok(Some((repo_name, repo_path)))
    } else {
        Ok(None)
    }
}

fn resolve_canonical_capability_target(
    prefixes: &[PathBuf],
    path: &Path,
) -> Result<Option<PathBuf>, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let target = canonicalize_target(&cwd, path)?;
    for prefix in prefixes {
        let allowed = canonicalize_target(&cwd, prefix)?;
        if !allowed.as_os_str().is_empty() && target.starts_with(&allowed) {
            return Ok(Some(target));
        }
    }
    Ok(None)
}

fn resolve_host_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn canonicalize_target(cwd: &Path, path: &Path) -> Result<PathBuf, String> {
    let absolute = resolve_host_path(cwd, path);
    let mut cursor = absolute.as_path();
    let mut missing = Vec::new();

    loop {
        match std::fs::symlink_metadata(cursor) {
            Ok(_) => {
                let mut canonical = std::fs::canonicalize(cursor).map_err(|e| {
                    format!("Failed to canonicalize plugin target {:?}: {}", cursor, e)
                })?;
                for component in missing.iter().rev() {
                    canonical.push(component);
                }
                return Ok(canonical);
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Failed to stat plugin target {:?}: {}", cursor, e)),
        }
        let Some(file_name) = cursor.file_name() else {
            return Err(format!(
                "Plugin target {:?} has no existing ancestor inside project root",
                absolute
            ));
        };
        missing.push(file_name.to_os_string());
        cursor = cursor.parent().ok_or_else(|| {
            format!(
                "Plugin target {:?} has no existing ancestor inside project root",
                absolute
            )
        })?;
    }
}

fn is_project_ledger_relative_path(rel_path: &str) -> bool {
    let parts: Vec<_> = rel_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    parts.first() == Some(&"ledger")
}

fn is_projection_workspace_managed_relative_path(repo_path: &str) -> bool {
    is_internal_repo_relative_path(repo_path) || is_projection_workspace_note_path(repo_path)
}

fn is_projection_workspace_note_path(repo_path: &str) -> bool {
    !repo_path.is_empty()
        && !is_internal_repo_relative_path(repo_path)
        && repo_path.ends_with(".md")
}

fn is_internal_repo_relative_path(repo_path: &str) -> bool {
    repo_path
        .split('/')
        .filter(|part| !part.is_empty())
        .any(crate::utils::notegit::is_internal_repo_segment)
}

fn repo_workspace_relative_for_canonical_target(
    manager: &crate::ledger::RepoManager,
    cwd: &Path,
    target: &Path,
) -> Result<Option<(String, String)>, String> {
    let repo_names = manager
        .list_local_repo_names_for_execution()
        .map_err(|e| e.to_string())?;
    for repo_name in repo_names {
        let root = manager
            .local_repo_workspace_root(&repo_name)
            .map_err(|e| e.to_string())?;
        let root = canonicalize_target(cwd, &root)?;
        if let Ok(repo_relative) = target.strip_prefix(&root) {
            return Ok(Some((
                repo_name,
                path_to_forward_slash(repo_relative)
                    .trim_matches('/')
                    .to_string(),
            )));
        }
    }
    Ok(None)
}

fn canonical_project_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    std::fs::canonicalize(&cwd)
        .map_err(|e| format!("Failed to canonicalize project root {:?}: {}", cwd, e))
}

fn project_relative_from_canonical(cwd: &Path, target: &Path) -> Option<String> {
    target.strip_prefix(cwd).ok().map(path_to_forward_slash)
}

#[cfg(test)]
mod tests;
