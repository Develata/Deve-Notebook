use crate::utils::path::to_forward_slash;

pub(super) fn normalize_doc_path(path: &str) -> String {
    to_forward_slash(path).trim_start_matches("./").to_string()
}

pub(super) fn title_from_path(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .strip_suffix(".md")
        .unwrap_or_else(|| path.rsplit('/').next().unwrap_or(path))
        .to_string()
}

pub(super) fn normalize_relative_target(source_path: &str, target: &str) -> String {
    let normalized = normalize_doc_path(target);
    if normalized.starts_with('/') {
        return normalized.trim_start_matches('/').to_string();
    }
    let base = source_path
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or_default();
    let joined = if base.is_empty() {
        normalized
    } else {
        format!("{base}/{normalized}")
    };
    collapse_path_segments(&joined)
}

pub(super) fn strip_anchor(target: &str) -> &str {
    target.split('#').next().unwrap_or(target)
}

fn collapse_path_segments(path: &str) -> String {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}
