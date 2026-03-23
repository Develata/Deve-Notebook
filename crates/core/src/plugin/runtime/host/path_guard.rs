use crate::utils::path::path_to_forward_slash;
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

fn canonicalize_target(cwd: &Path, path: &Path) -> Result<PathBuf, String> {
    let absolute = normalize_host_path(&resolve_host_path(cwd, path));
    let mut cursor = absolute.as_path();
    let mut missing = Vec::new();

    loop {
        let exists = cursor
            .try_exists()
            .map_err(|e| format!("Failed to stat plugin target {:?}: {}", cursor, e))?;
        if exists {
            let mut canonical = std::fs::canonicalize(cursor)
                .map_err(|e| format!("Failed to canonicalize plugin target {:?}: {}", cursor, e))?;
            for component in missing.iter().rev() {
                canonical.push(component);
            }
            return Ok(canonical);
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

#[cfg(test)]
mod tests {
    use super::{is_ledger_managed_write_target, project_relative_path};
    use std::path::Path;
    use tempfile::tempdir;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[cfg(unix)]
    #[test]
    fn ledger_managed_detection_fails_closed_through_symlink() {
        let dir = tempdir().expect("tempdir");
        let cwd = std::env::current_dir().expect("cwd");
        let vault = dir.path().join("vault/default/notes");
        std::fs::create_dir_all(&vault).expect("mkdir");
        let target = vault.join("a.md");
        std::fs::write(&target, "hello").expect("write");
        let alias_dir = dir.path().join("tmp");
        std::fs::create_dir_all(&alias_dir).expect("mkdir alias");
        let alias = alias_dir.join("alias.md");
        symlink(&target, &alias).expect("symlink");

        std::env::set_current_dir(dir.path()).expect("set cwd");
        let detected = is_ledger_managed_write_target(Path::new("tmp/alias.md"))
            .expect("managed detection should succeed");
        std::env::set_current_dir(cwd).expect("restore cwd");

        assert!(detected);
    }

    #[cfg(unix)]
    #[test]
    fn project_relative_path_uses_canonical_target_location() {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault/default/notes");
        std::fs::create_dir_all(&vault).expect("mkdir");
        let target = vault.join("a.md");
        std::fs::write(&target, "hello").expect("write");
        let alias_dir = dir.path().join("tmp");
        std::fs::create_dir_all(&alias_dir).expect("mkdir alias");
        let alias = alias_dir.join("alias.md");
        symlink(&target, &alias).expect("symlink");

        let rel = project_relative_path(dir.path(), Path::new("tmp/alias.md"))
            .expect("canonical relative path")
            .expect("inside project root");

        assert_eq!(rel, "vault/default/notes/a.md");
    }
}
