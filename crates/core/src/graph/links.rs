use super::GraphLinkKind;
use super::path::{normalize_relative_target, strip_anchor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkCandidate {
    pub(super) text: String,
    target: String,
    pub(super) kind: GraphLinkKind,
}

pub(super) fn extract_link_candidates(content: &str) -> Vec<LinkCandidate> {
    let mut links = Vec::new();
    extract_wiki_links(content, &mut links);
    extract_markdown_links(content, &mut links);
    links
}

pub(super) fn resolve_link_target(source_path: &str, candidate: &LinkCandidate) -> String {
    let target = strip_anchor(&candidate.target).trim();
    let target = match candidate.kind {
        GraphLinkKind::Wiki if !target.ends_with(".md") => format!("{target}.md"),
        _ => target.to_string(),
    };
    normalize_relative_target(source_path, &target)
}

fn extract_wiki_links(content: &str, links: &mut Vec<LinkCandidate>) {
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let raw = rest[..end].trim();
        if !raw.is_empty() {
            let (target, text) = split_wiki_link(raw);
            links.push(LinkCandidate {
                text,
                target,
                kind: GraphLinkKind::Wiki,
            });
        }
        rest = &rest[end + 2..];
    }
}

fn split_wiki_link(raw: &str) -> (String, String) {
    let mut parts = raw.splitn(2, '|');
    let target = parts.next().unwrap_or_default().trim().to_string();
    let text = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(target.as_str())
        .to_string();
    (target, text)
}

fn extract_markdown_links(content: &str, links: &mut Vec<LinkCandidate>) {
    let bytes = content.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let Some(label_start) = content[idx..].find('[').map(|offset| idx + offset) else {
            break;
        };
        if content[label_start..].starts_with("[[") {
            idx = label_start + 2;
            continue;
        }
        let Some(label_end) = content[label_start + 1..]
            .find(']')
            .map(|offset| label_start + 1 + offset)
        else {
            break;
        };
        if !content[label_end + 1..].starts_with('(') {
            idx = label_end + 1;
            continue;
        }
        let target_start = label_end + 2;
        let Some(target_end) = content[target_start..]
            .find(')')
            .map(|offset| target_start + offset)
        else {
            break;
        };
        let text = content[label_start + 1..label_end].trim();
        let target = content[target_start..target_end].trim();
        if is_local_markdown_target(target) {
            links.push(LinkCandidate {
                text: text.to_string(),
                target: target.to_string(),
                kind: GraphLinkKind::Markdown,
            });
        }
        idx = target_end + 1;
    }
}

fn is_local_markdown_target(target: &str) -> bool {
    let target = target.trim();
    !target.is_empty()
        && !target.starts_with('#')
        && !target.contains("://")
        && !target.starts_with("mailto:")
        && strip_anchor(target).ends_with(".md")
}
