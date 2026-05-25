//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 17_tech_stack#graph-visualization
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
        Locale::En => "Graph projection request failed. Use `deve graph` for CLI diagnostics.",
        Locale::Zh => "图谱投影请求失败。可使用 `deve graph` 进行 CLI 诊断。",
    }
}

pub fn graph_projection_local_only(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Graph projection is currently available for local repo scope only.",
        Locale::Zh => "当前图谱投影只支持本地仓库作用域。",
    }
}

pub fn graph_projection_blocked(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Graph projection is blocked until Source Control read scope is ready.",
        Locale::Zh => "Source Control 读取作用域就绪前，图谱投影会被阻断。",
    }
}

pub fn graph_projection_degraded(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Graph projection is degraded; CLI export requires explicit `--allow-degraded-projection`."
        }
        Locale::Zh => "图谱投影处于降级状态；CLI 导出必须显式使用 `--allow-degraded-projection`。",
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
    use super::{
        graph_edges, graph_nodes, graph_projection_blocked, graph_projection_degraded,
        graph_readonly_note, graph_unresolved_links,
    };
    use crate::i18n::Locale;

    #[test]
    fn graph_panel_copy_is_localized() {
        assert_eq!(graph_nodes(Locale::En), "Nodes");
        assert_eq!(graph_edges(Locale::Zh), "边");
        assert_eq!(graph_unresolved_links(Locale::En), "Unresolved");
        assert!(graph_readonly_note(Locale::Zh).contains("只读"));
        assert!(graph_projection_blocked(Locale::En).contains("blocked"));
        assert!(graph_projection_degraded(Locale::Zh).contains("降级"));
    }
}
