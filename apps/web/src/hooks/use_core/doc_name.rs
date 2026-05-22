//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!
use std::collections::HashSet;

pub fn next_untitled_doc_path<'a, I>(paths: I, parent: Option<&str>) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let prefix = parent
        .map(normalize_parent_prefix)
        .filter(|path| !path.is_empty())
        .unwrap_or_default();
    let mut child_names = Vec::new();

    for path in paths {
        let normalized = deve_core::utils::path::to_forward_slash(path);
        let Some(rest) = normalized.strip_prefix(&prefix) else {
            continue;
        };
        if !rest.is_empty() && !rest.contains('/') {
            child_names.push(rest.to_string());
        }
    }

    let name = next_untitled_doc_name(child_names.iter().map(|path| path.as_str()));
    format!("{}{}", prefix, name)
}

pub fn next_untitled_doc_name<'a, I>(paths: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let mut untitled_taken = false;
    let mut numbered = HashSet::new();

    for path in paths {
        if path == "Untitled.md" {
            untitled_taken = true;
            continue;
        }

        let Some(stem) = path.strip_prefix("Untitled ") else {
            continue;
        };
        let Some(number) = stem.strip_suffix(".md") else {
            continue;
        };
        let Ok(parsed) = number.parse::<u32>() else {
            continue;
        };
        if parsed >= 2 {
            numbered.insert(parsed);
        }
    }

    if !untitled_taken {
        return "Untitled.md".to_string();
    }

    let mut next = 2;
    while numbered.contains(&next) {
        next += 1;
    }
    format!("Untitled {}.md", next)
}

fn normalize_parent_prefix(parent: &str) -> String {
    let normalized = deve_core::utils::path::to_forward_slash(parent);
    let trimmed = normalized.trim().trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}/")
    }
}

#[cfg(test)]
mod tests {
    use super::{next_untitled_doc_name, next_untitled_doc_path};

    #[test]
    fn prefers_plain_untitled_when_available() {
        assert_eq!(
            next_untitled_doc_name(["notes/a.md", "Untitled 2.md"]),
            "Untitled.md"
        );
    }

    #[test]
    fn increments_when_plain_untitled_exists() {
        assert_eq!(
            next_untitled_doc_name(["Untitled.md", "notes/a.md"]),
            "Untitled 2.md"
        );
    }

    #[test]
    fn fills_first_gap_in_numbered_sequence() {
        assert_eq!(
            next_untitled_doc_name([
                "Untitled.md",
                "Untitled 2.md",
                "Untitled 4.md",
                "nested/Untitled 3.md",
            ]),
            "Untitled 3.md"
        );
    }

    #[test]
    fn builds_top_level_untitled_path() {
        assert_eq!(
            next_untitled_doc_path(["notes/a.md", "Untitled.md"], None),
            "Untitled 2.md"
        );
    }

    #[test]
    fn builds_nested_untitled_path_from_direct_children_only() {
        assert_eq!(
            next_untitled_doc_path(
                [
                    "notes/Untitled.md",
                    "notes/Untitled 2.md",
                    "notes/nested/Untitled 3.md",
                    "Untitled.md",
                ],
                Some("notes/")
            ),
            "notes/Untitled 3.md"
        );
    }

    #[test]
    fn normalizes_windows_parent_prefix() {
        assert_eq!(
            next_untitled_doc_path(["notes\\Untitled.md"], Some("notes\\")),
            "notes/Untitled 2.md"
        );
    }
}
