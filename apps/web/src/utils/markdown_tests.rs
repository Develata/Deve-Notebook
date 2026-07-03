//! plan_ref:
//!   - 10_rendering#markdown-render-whitelist

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
