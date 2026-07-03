// apps/web/src/utils/markdown.rs
//! Lightweight Markdown renderer with HTML filtering and secure link handling.
//! plan_ref:
//!   - 10_rendering#markdown-render-whitelist

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd, html};

pub fn render_markdown(source: &str, apply_label: Option<&str>) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut out = String::new();
    let mut buffer: Vec<Event> = Vec::new();
    let mut iter = Parser::new_ext(source, options)
        .filter(|event| match event {
            Event::Html(tag) | Event::InlineHtml(tag) => {
                let t = tag.trim();
                t.eq_ignore_ascii_case("<br>")
                    || t.eq_ignore_ascii_case("<br/>")
                    || t.eq_ignore_ascii_case("<br />")
            }
            _ => true,
        })
        .peekable();

    while let Some(event) = iter.next() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                if !buffer.is_empty() {
                    html::push_html(&mut out, buffer.drain(..));
                }
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                let mut code = String::new();
                for ev in iter.by_ref() {
                    match ev {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(t) | Event::Code(t) => code.push_str(&t),
                        Event::SoftBreak | Event::HardBreak => code.push('\n'),
                        _ => {}
                    }
                }
                out.push_str(&render_code_block(&code, &lang, apply_label));
            }
            // Intercept links: add target="_blank" and security attributes
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            }) => {
                if !buffer.is_empty() {
                    html::push_html(&mut out, buffer.drain(..));
                }
                render_link_open(&mut out, &dest_url, &title, link_type);
            }
            Event::End(TagEnd::Link) => {
                if !buffer.is_empty() {
                    html::push_html(&mut out, buffer.drain(..));
                }
                out.push_str("</a>");
            }
            other => buffer.push(other),
        }
    }

    if !buffer.is_empty() {
        html::push_html(&mut out, buffer.drain(..));
    }

    out
}

fn render_code_block(code: &str, lang: &str, apply_label: Option<&str>) -> String {
    let escaped = escape_html(code);
    let lang_class = if lang.is_empty() {
        "".to_string()
    } else {
        format!("language-{}", lang)
    };
    let escaped_lang_class = escape_html(&lang_class);

    match apply_label {
        Some(label) => {
            let encoded = STANDARD.encode(code.as_bytes());
            format!(
                "<div class=\"markdown-code-block\"><div class=\"code-toolbar\"><button class=\"apply-code\" data-code=\"{}\">{}</button></div><pre><code class=\"{}\">{}</code></pre></div>",
                encoded,
                escape_html(label),
                escaped_lang_class,
                escaped
            )
        }
        None => format!(
            "<div class=\"markdown-code-block\"><pre><code class=\"{}\">{}</code></pre></div>",
            escaped_lang_class, escaped
        ),
    }
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Renders secure link opening tag with target="_blank" and rel="noopener noreferrer".
///
/// # Security
/// External links MUST include `rel="noopener noreferrer"` to prevent:
/// - `window.opener` attacks (tabnabbing)
/// - Referrer leakage
fn render_link_open(out: &mut String, url: &str, title: &str, _link_type: LinkType) {
    let href = sanitized_href(url);
    let escaped_url = escape_html(href);
    out.push_str("<a href=\"");
    out.push_str(&escaped_url);
    out.push_str("\" target=\"_blank\" rel=\"noopener noreferrer\"");
    if !title.is_empty() {
        out.push_str(" title=\"");
        out.push_str(&escape_html(title));
        out.push('"');
    }
    out.push('>');
}

fn sanitized_href(url: &str) -> &str {
    let trimmed = url.trim();
    if is_safe_href(trimmed) { trimmed } else { "#" }
}

fn is_safe_href(url: &str) -> bool {
    if url.chars().any(char::is_control) {
        return false;
    }

    let Some(colon) = url.find(':') else {
        return true;
    };

    let first_path_delim = [url.find('/'), url.find('?'), url.find('#')]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(usize::MAX);
    if colon > first_path_delim {
        return true;
    }

    matches!(
        &url[..colon].to_ascii_lowercase()[..],
        "http" | "https" | "mailto"
    )
}

#[cfg(test)]
mod tests {
    use super::render_markdown;

    #[test]
    fn code_block_omits_apply_button_without_label() {
        let html = render_markdown("```md\nhello\n```", None);
        assert!(!html.contains("apply-code"));
        assert!(!html.contains("data-code"));
        assert!(html.contains("hello"));
    }

    #[test]
    fn code_block_includes_apply_button_with_label() {
        let html = render_markdown("```md\nhello\n```", Some("Apply"));
        assert!(html.contains("apply-code"));
        assert!(html.contains("data-code"));
        assert!(html.contains(">Apply</button>"));
    }

    #[test]
    fn html_filter_allows_br_only() {
        let html = render_markdown("a<br>b<script>alert(1)</script><div>x</div>", None);
        assert!(html.contains("<br"));
        assert!(!html.contains("<script"));
        assert!(html.contains("alert(1)"));
        assert!(!html.contains("<div"));
    }

    #[test]
    fn link_rendering_adds_blank_target_and_rel() {
        let html = render_markdown("[site](https://example.com \"safe\")", None);
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("rel=\"noopener noreferrer\""));
        assert!(html.contains("title=\"safe\""));
    }

    #[test]
    fn link_rendering_rejects_script_scheme() {
        let html = render_markdown("[bad](javascript:alert(1))", None);
        assert!(html.contains("href=\"#\""));
        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn unsupported_highlight_syntax_stays_plain_text() {
        let html = render_markdown("==mark==", None);
        assert!(html.contains("==mark=="));
        assert!(!html.contains("<mark"));
    }

    #[test]
    fn chat_math_keeps_tex_delimiters_for_dom_projection() {
        let html = render_markdown("inline $a^2$ and $$b^2$$", None);
        assert!(html.contains("$a^2$"));
        assert!(html.contains("$$b^2$$"));
        assert!(!html.contains("katex"));
    }

    #[test]
    fn chat_math_keeps_code_block_math_literal_inside_pre_code() {
        let html = render_markdown("```text\n$not_math$\n```", None);
        assert!(html.contains("markdown-code-block"));
        assert!(html.contains("<pre><code class=\"language-text\">"));
        assert!(html.contains("$not_math$"));
    }

    #[test]
    fn code_block_language_class_is_escaped() {
        let html = render_markdown("```rust\" onclick=\"alert(1)\nfn main() {}\n```", None);
        assert!(html.contains("class=\"language-rust&quot;"));
        assert!(!html.contains("onclick="));
        assert!(html.contains("fn main() {}"));
    }

    #[test]
    fn empty_atx_headings_emit_distinct_html_levels() {
        let html = render_markdown("#\n##\n###", None);
        for tag in ["<h1></h1>", "<h2></h2>", "<h3></h3>"] {
            assert!(html.contains(tag));
        }
    }

    #[test]
    fn markdown_heading_modes_keep_nonempty_atx_rows_tall() {
        let html = render_markdown("# s\n## s\n### s", None);
        for tag in ["<h1>s</h1>", "<h2>s</h2>", "<h3>s</h3>"] {
            assert!(html.contains(tag));
        }
        assert!(!html.contains("# s"));
        assert!(!html.contains("## s"));
        assert!(!html.contains("### s"));

        let block_styling = include_str!("../../js/extensions/block_styling.js");
        assert!(block_styling.contains("ATX_HEADING_LINE_RE"));
        assert!(block_styling.contains("ACTIVE_CJK_ATX_HEADING_LINE_RE"));
        assert!(block_styling.contains("cm-heading-line-${headingLevel}"));

        let typography = include_str!("../../style/_typography.css");
        assert!(typography.contains(".cm-content .cm-line.cm-heading-line.cm-activeLine"));
        assert!(typography.contains("--deve-heading-line-box"));
        assert!(typography.contains("--deve-heading-inline-line-height"));
        assert!(typography.contains(".markdown-body h1"));
    }
}
