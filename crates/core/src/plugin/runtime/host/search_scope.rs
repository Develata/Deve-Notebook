use crate::utils::path::path_to_forward_slash;
use std::path::{Path, PathBuf};

pub(super) fn resolve_search_root(root: &Path, path: &str) -> Result<PathBuf, String> {
    let search_root = if path.is_empty() {
        root.to_path_buf()
    } else {
        root.join(path)
    };
    ensure_within_root(root, &search_root, "search root")
}

pub(super) fn relative_search_path(root: &Path, path: &Path) -> Result<String, String> {
    let normalized_root = normalize_host_path(root);
    let normalized_path = normalize_host_path(path);
    normalized_path
        .strip_prefix(&normalized_root)
        .map(path_to_forward_slash)
        .map_err(|_| {
            format!(
                "Search path escaped project root: {}",
                normalized_path.display()
            )
        })
}

fn ensure_within_root(root: &Path, path: &Path, context: &str) -> Result<PathBuf, String> {
    let normalized_root = normalize_host_path(root);
    let normalized_path = normalize_host_path(path);
    normalized_path
        .strip_prefix(&normalized_root)
        .map_err(|_| {
            format!(
                "{context} escaped project root: {}",
                normalized_path.display()
            )
        })?;
    Ok(normalized_path)
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

#[cfg(test)]
mod tests {
    use super::{relative_search_path, resolve_search_root};
    use std::path::Path;

    #[test]
    fn resolve_search_root_rejects_parent_escape() {
        let root = Path::new("/tmp/project");
        let err = resolve_search_root(root, "../outside").unwrap_err();
        assert!(err.contains("escaped project root"));
    }

    #[test]
    fn relative_search_path_rejects_path_outside_root() {
        let root = Path::new("/tmp/project");
        let err = relative_search_path(root, Path::new("/tmp/outside/file.md")).unwrap_err();
        assert!(err.contains("escaped project root"));
    }
}
