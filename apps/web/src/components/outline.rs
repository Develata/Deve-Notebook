// apps\web\src\components
//! # Outline 组件 (Outline Component)
//!
//! 显示文档大纲，基于 Markdown 标题解析。

use crate::components::outline_render::render_outline_inline;
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
        let trimmed = line.trim();

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

        if trimmed.starts_with('#') {
            let mut level = 0;
            for c in trimmed.chars() {
                if c == '#' {
                    level += 1;
                } else {
                    break;
                }
            }

            // 仅当后面有空格且级别 <= 6 时转换
            if level > 0 && level <= 6 {
                // 检查下一个字符是否为空格
                let rest = &trimmed[level..];
                if rest.starts_with(' ') {
                    headers.push(HeaderNode {
                        level,
                        text: rest.trim().to_string(),
                        line: i + 1,
                    });
                }
            }
        }
    }

    headers
}

#[component]
pub fn Outline(content: ReadSignal<String>, on_scroll: Callback<usize>) -> impl IntoView {
    let headers = Memo::new(move |_| parse_headers(&content.get()));

    view! {
        <div class="h-full overflow-y-auto py-3 px-2 select-none">
                <div class="font-bold text-gray-500 mb-2 px-2 text-[10px] uppercase tracking-wider">
                "Outline"
            </div>
            <For
                each=move || headers.get()
                key=|h| (h.line, h.text.clone())
                children=move |header| {
                    let on_click = on_scroll.clone();
                    let line = header.line;

                    let text = header.text.clone();
                    let title_text = text.clone();
                    let rendered = render_outline_inline(&text);
                    let padding = format!("padding-left: {}px", (header.level - 1) * 10 + 8);

                    view! {
                        <div
                            class="min-h-8 py-1.5 pr-2 text-xs text-gray-600 hover:bg-gray-50 hover:text-gray-900 active:bg-gray-100 cursor-pointer rounded transition-colors truncate flex items-center"
                            style={padding}
                            on:click=move |_| on_click.run(line)
                            title={title_text}
                        >
                            {rendered}
                        </div>
                    }
                }
            />
        </div>
    }
}
