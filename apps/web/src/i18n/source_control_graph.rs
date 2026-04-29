//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!   - 14_tech_stack#graph-visualization
//!
//! Graph panel translation strings for Source Control.

use super::Locale;

pub fn loading_graph(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading graph projection...",
        Locale::Zh => "正在加载图谱投影……",
    }
}

pub fn graph_projection_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Graph projection is unavailable. Use `deve graph` for CLI diagnostics.",
        Locale::Zh => "图谱投影暂不可用。可使用 `deve graph` 进行 CLI 诊断。",
    }
}

pub fn graph_projection_local_only(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Graph projection is currently available for local repo scope only.",
        Locale::Zh => "当前图谱投影只支持本地仓库作用域。",
    }
}

pub fn graph_projection_empty(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No graph projection loaded yet.",
        Locale::Zh => "尚未加载图谱投影。",
    }
}

pub fn graph_nodes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Nodes",
        Locale::Zh => "节点",
    }
}

pub fn graph_edges(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Edges",
        Locale::Zh => "边",
    }
}

pub fn graph_unresolved_links(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unresolved",
        Locale::Zh => "未解析",
    }
}

pub fn graph_readonly_note(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Read-only projection summary. Canvas rendering remains future work.",
        Locale::Zh => "只读投影摘要。Canvas 渲染仍属后续工作。",
    }
}

#[cfg(test)]
mod tests {
    use super::{graph_edges, graph_nodes, graph_readonly_note, graph_unresolved_links};
    use crate::i18n::Locale;

    #[test]
    fn graph_panel_copy_is_localized() {
        assert_eq!(graph_nodes(Locale::En), "Nodes");
        assert_eq!(graph_edges(Locale::Zh), "边");
        assert_eq!(graph_unresolved_links(Locale::En), "Unresolved");
        assert!(graph_readonly_note(Locale::Zh).contains("只读"));
    }
}
