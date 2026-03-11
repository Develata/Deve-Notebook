use crate::utils::path::path_to_forward_slash;
use std::path::{Path, PathBuf};

pub(super) fn project_relative_path(cwd: &Path, path: &Path) -> Option<String> {
    let cwd = normalize_host_path(cwd);
    let path = normalize_host_path(&resolve_host_path(cwd.as_path(), path));
    path.strip_prefix(&cwd).ok().map(path_to_forward_slash)
}

pub(super) fn is_ledger_managed_write_target(path: &Path) -> Result<bool, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(project_relative_path(&cwd, path).is_some_and(|rel| is_ledger_managed_relative_path(&rel)))
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

fn resolve_host_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn normalize_host_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(..) | std::path::Component::RootDir => {
                normalized.push(component.as_os_str())
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(segment) => normalized.push(segment),
        }
    }
    normalized
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
        if parts.iter().skip(2).any(|part| *part == ".notegit") {
            return true;
        }
        return rel_path.ends_with(".md");
    }
    false
}
