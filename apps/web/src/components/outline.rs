// apps\web\src\components
//! plan_ref:
//!   - 10_rendering#outline-projection
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! # Outline 组件 (Outline Component)
//!
//! 显示文档大纲，基于 Markdown 标题解析。

use crate::components::outline_render::render_outline_inline;
use crate::components::touch_feedback::interactive_item_state_class;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct HeaderNode {
    pub level: usize,
    pub text: String,
    pub line: usize, // 1-based 行号
}

// 简单的 Markdown 标题解析器。
// 返回扁平列表。我们可以通过 padding 来渲染缩进。
pub fn parse_headers(content: &str) -> Vec<HeaderNode> {
    let mut headers = Vec::new();
    let mut in_code_block = false;

    for (i, line) in content.lines().enumerate() {
        if is_fence_line(line) {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        if let Some((level, text)) = parse_atx_heading(line) {
            headers.push(HeaderNode {
                level,
                text,
                line: i + 1,
            });
        }
    }

    headers
}

fn parse_atx_heading(line: &str) -> Option<(usize, String)> {
    if is_indented_code_line(line) {
        return None;
    }

    let trimmed = line.trim_start_matches(' ');
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return None;
    }

    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if level == 0 || level > 6 {
        return None;
    }

    let rest = &trimmed[level..];
    if rest.starts_with('#') {
        return None;
    }
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }

    let text = strip_atx_closing_sequence(rest.trim_start_matches(|ch| ch == ' ' || ch == '\t'));
    Some((level, text.to_string()))
}

fn strip_atx_closing_sequence(rest: &str) -> &str {
    let trimmed = rest.trim_end_matches(|ch| ch == ' ' || ch == '\t');
    if !trimmed.ends_with('#') {
        return trimmed;
    }

    let closing_start = trimmed
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| (ch != '#').then_some(idx + ch.len_utf8()))
        .unwrap_or(0);
    if closing_start == 0 {
        return "";
    }

    let before_closing = &trimmed[..closing_start];
    if before_closing.ends_with(' ') || before_closing.ends_with('\t') {
        before_closing.trim_end_matches(|ch| ch == ' ' || ch == '\t')
    } else {
        trimmed
    }
}

fn is_fence_line(line: &str) -> bool {
    if is_indented_code_line(line) {
        return false;
    }

    let trimmed = line.trim_start_matches(' ');
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn is_indented_code_line(line: &str) -> bool {
    line.starts_with("    ") || line.starts_with('\t')
}

#[component]
pub fn Outline(content: ReadSignal<String>, on_scroll: Callback<usize>) -> impl IntoView {
    let headers = Memo::new(move |_| parse_headers(&content.get()));
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <div class="h-full overflow-y-auto py-3 px-2 select-none">
                <div class="font-bold text-muted mb-2 px-2 text-[10px] uppercase tracking-wider">
                {move || t::sidebar::outline(locale.get())}
            </div>
            <For
                each=move || headers.get()
                key=|h| (h.line, h.text.clone())
                children=move |header| {
                    let on_click = on_scroll.clone();
                    let line = header.line;

                    let text = header.text.clone();
                    let rendered = if text.is_empty() {
                        vec![view! { <span class="text-muted italic">{t::sidebar::empty_outline_heading(locale.get(), line)}</span> }.into_any()]
                    } else {
                        render_outline_inline(&text)
                    };
                    let title_text = if text.is_empty() {
                        t::sidebar::empty_outline_heading(locale.get(), line)
                    } else {
                        text.clone()
                    };
                    let aria_text = title_text.clone();
                    let padding = format!("padding-left: {}px", (header.level - 1) * 10 + 8);

                    view! {
                        <button
                            type="button"
                            data-deve-outline-heading-item="true"
                            data-deve-outline-heading-line=line.to_string()
                            class=format!(
                                "w-full min-h-8 py-1.5 pr-2 text-left text-xs cursor-pointer rounded transition-colors truncate flex items-center bg-transparent border-0 {}",
                                interactive_item_state_class(false, true),
                            )
                            style={padding}
                            on:click=move |_| on_click.run(line)
                            title={title_text}
                            aria-label={aria_text}
                        >
                            {rendered}
                        </button>
                    }
                }
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{HeaderNode, parse_headers, strip_atx_closing_sequence};

    #[test]
    fn outline_atx_heading_scan_supports_empty_tab_and_closing_sequence() {
        let headers = parse_headers("#\n#\tTabbed\n## title ##\n### title ###   \n# ###\n");

        assert_eq!(
            headers,
            vec![
                HeaderNode {
                    level: 1,
                    text: String::new(),
                    line: 1,
                },
                HeaderNode {
                    level: 1,
                    text: "Tabbed".to_string(),
                    line: 2,
                },
                HeaderNode {
                    level: 2,
                    text: "title".to_string(),
                    line: 3,
                },
                HeaderNode {
                    level: 3,
                    text: "title".to_string(),
                    line: 4,
                },
                HeaderNode {
                    level: 1,
                    text: String::new(),
                    line: 5,
                },
            ]
        );
    }

    #[test]
    fn outline_atx_heading_scan_skips_code_and_invalid_openers() {
        let headers = parse_headers(
            "```md\n# code\n```\n    # indented\n#invalid\n####### too many\n ### ok\n",
        );

        assert_eq!(
            headers,
            vec![HeaderNode {
                level: 3,
                text: "ok".to_string(),
                line: 7,
            }]
        );
    }

    #[test]
    fn outline_atx_closing_sequence_requires_separator() {
        assert_eq!(strip_atx_closing_sequence("title ###"), "title");
        assert_eq!(strip_atx_closing_sequence("title###"), "title###");
        assert_eq!(strip_atx_closing_sequence("###"), "");
    }
}
