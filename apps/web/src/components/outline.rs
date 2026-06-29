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

// 简单的 Markdown 标题解析器
// 返回扁平列表。我们可以通过 padding 来渲染缩进。
pub fn parse_headers(content: &str) -> Vec<HeaderNode> {
    let mut headers = Vec::new();
    let mut in_code_block = false;

    for (i, line) in content.lines().enumerate() {
        let leading_spaces = line.chars().take_while(|c| *c == ' ').count();
        let trimmed = line.trim_start();

        // Check for code block fences (``` or ~~~)
        // Note: This is a simplified check. It assumes the fence is at the start of the line (after trim).
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }

        // Skip content inside code blocks
        if in_code_block {
            continue;
        }

        if leading_spaces <= 3
            && let Some((level, text)) = parse_atx_heading(trimmed)
        {
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
    if !line.starts_with('#') {
        return None;
    }

    let level = line.bytes().take_while(|b| *b == b'#').count();
    if level == 0 || level > 6 {
        return None;
    }

    let rest = &line[level..];
    if rest.chars().next().is_some_and(|ch| !ch.is_whitespace()) {
        return None;
    }

    Some((level, strip_atx_closing_sequence(rest).to_string()))
}

fn strip_atx_closing_sequence(rest: &str) -> &str {
    let text = rest.trim();
    let without_trailing_space = text.trim_end_matches([' ', '\t']);
    let without_closing_hashes = without_trailing_space.trim_end_matches('#');
    if without_closing_hashes.len() == without_trailing_space.len() {
        return text;
    }

    if without_closing_hashes.is_empty()
        || without_closing_hashes
            .chars()
            .last()
            .is_some_and(|ch| ch.is_whitespace())
    {
        without_closing_hashes.trim_end_matches([' ', '\t'])
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::{HeaderNode, parse_headers};

    #[test]
    fn outline_atx_heading_scan_supports_empty_tab_and_closing_sequence() {
        let headers = parse_headers(
            "\
#
##\tTabbed
### Title ###
#### Keep#
# ###
",
        );

        assert_eq!(
            headers,
            vec![
                HeaderNode {
                    level: 1,
                    text: String::new(),
                    line: 1,
                },
                HeaderNode {
                    level: 2,
                    text: "Tabbed".to_string(),
                    line: 2,
                },
                HeaderNode {
                    level: 3,
                    text: "Title".to_string(),
                    line: 3,
                },
                HeaderNode {
                    level: 4,
                    text: "Keep#".to_string(),
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
            "\
```md
# fenced
```
####### too-many
    # indented-code
   ## allowed
",
        );

        assert_eq!(
            headers,
            vec![HeaderNode {
                level: 2,
                text: "allowed".to_string(),
                line: 6,
            }]
        );
    }
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
                    let title_text = text.clone();
                    let aria_text = text.clone();
                    let rendered = render_outline_inline(&text);
                    let padding = format!("padding-left: {}px", (header.level - 1) * 10 + 8);

                    view! {
                        <button
                            type="button"
                            data-deve-outline-heading-item="true"
                            data-deve-outline-heading-level=header.level.to_string()
                            data-deve-outline-heading-line=line.to_string()
                            class=format!(
                                "min-h-8 w-full py-1.5 pr-2 text-left text-xs cursor-pointer rounded transition-colors truncate flex items-center {}",
                                interactive_item_state_class(false, true),
                            )
                            style={padding}
                            on:click=move |_| on_click.run(line)
                            title=move || outline_heading_label(locale.get(), &title_text, line)
                            aria-label=move || outline_heading_label(locale.get(), &aria_text, line)
                        >
                            {rendered}
                        </button>
                    }
                }
            />
        </div>
    }
}

fn outline_heading_label(locale: Locale, text: &str, line: usize) -> String {
    if text.is_empty() {
        t::sidebar::empty_outline_heading(locale, line)
    } else {
        text.to_string()
    }
}
