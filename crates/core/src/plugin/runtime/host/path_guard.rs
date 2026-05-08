//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
use crate::plugin::manifest::Capability;
use crate::utils::path::path_to_forward_slash;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub(super) fn project_relative_path(cwd: &Path, path: &Path) -> Result<Option<String>, String> {
    let cwd = std::fs::canonicalize(cwd)
        .map_err(|e| format!("Failed to canonicalize project root {:?}: {}", cwd, e))?;
    let path = canonicalize_target(cwd.as_path(), path)?;
    Ok(path.strip_prefix(&cwd).ok().map(path_to_forward_slash))
}

pub(super) fn is_ledger_managed_write_target(path: &Path) -> Result<bool, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(project_relative_path(&cwd, path)?.is_some_and(|rel| is_ledger_managed_relative_path(&rel)))
}

pub(super) fn is_capability_read_target(caps: &Capability, path: &Path) -> Result<bool, String> {
    if !caps.check_read(path) {
        return Ok(false);
    }
    is_canonical_capability_target(&caps.allow_fs_read, path)
}

pub(super) fn is_capability_write_target(caps: &Capability, path: &Path) -> Result<bool, String> {
    if !caps.check_write(path) {
        return Ok(false);
    }
    is_canonical_capability_target(&caps.allow_fs_write, path)
}

pub(super) fn split_managed_note_target(rel_path: &str) -> Option<(String, String)> {
    let parts: Vec<_> = rel_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 3 || parts[0] != "vault" || !rel_path.ends_with(".md") {
        return None;
    }
    let repo_name = parts[1].to_string();
    let repo_path = parts[2..].join("/");
    Some((repo_name, repo_path))
}

fn is_canonical_capability_target(prefixes: &[PathBuf], path: &Path) -> Result<bool, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let target = canonicalize_target(&cwd, path)?;
    prefixes.iter().try_fold(false, |matched, prefix| {
        if matched {
            return Ok(true);
        }
        let allowed = canonicalize_target(&cwd, prefix)?;
        Ok(!allowed.as_os_str().is_empty() && target.starts_with(&allowed))
    })
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

fn is_ledger_managed_relative_path(rel_path: &str) -> bool {
    let parts: Vec<_> = rel_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.first() == Some(&"ledger") {
        return true;
    }
    if parts.len() >= 3 && parts[0] == "vault" {
        if parts
            .iter()
            .skip(2)
            .any(|part| crate::utils::notegit::is_internal_repo_segment(part))
        {
            return true;
        }
        return rel_path.ends_with(".md");
    }
    false
}

#[cfg(test)]
#[path = "path_guard_test.rs"]
mod tests;
